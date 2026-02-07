use axum::{
    routing::{get, any},
    response::IntoResponse,
    Router,
    extract::{ws::{WebSocketUpgrade, WebSocket}, State, Request},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn};
use crate::routing::{RouteRule, Router as OrchixRouter};
use crate::interception::{InterceptionConfig, Interceptor};
use crate::streaming::StreamingAnalyzer;
use crate::config::{ServerConfig, SecurityConfig, CacheConfig};
use crate::auth::auth_middleware;
use crate::cache::{OrchixCache, CacheKey, CachedResponse};
use futures::stream;
use axum::response::sse::Sse;
use std::convert::Infallible;
use tokio_stream::StreamExt as _;
use std::time::Duration;
use bytes::Bytes;

pub struct AppState {
    pub router: OrchixRouter,
    pub interceptor: Interceptor,
    pub security: SecurityConfig,
    pub cache: OrchixCache,
    pub caching_config: CacheConfig,
}

pub async fn run_server(
    config: ServerConfig, 
    rules: Vec<RouteRule>,
    interception_config: InterceptionConfig,
    security_config: SecurityConfig,
    cache_config: CacheConfig,
) -> anyhow::Result<()> {
    // 状態の初期化
    let state = Arc::new(AppState {
        router: OrchixRouter::new(rules),
        interceptor: Interceptor::new(interception_config),
        security: security_config,
        cache: OrchixCache::new(&cache_config),
        caching_config: cache_config,
    });

    let auth_layer = axum::middleware::from_fn_with_state(state.clone(), auth_middleware);

    // HTTPルーター（Axum側）の設定
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler).layer(auth_layer.clone()))
        .route("/v1/stream_test", get(stream_test_handler).layer(auth_layer.clone()))
        .fallback(any(proxy_handler).layer(auth_layer))
        .with_state(state);

    // 設定値に基づいてアドレスを作成
    let addr_str = format!("{}:{}", config.host, config.port);
    let addr: SocketAddr = addr_str.parse()?;
    info!("listening on {}", addr);

    // サーバーの起動
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ヘルスチェック用ハンドラ
async fn health_check() -> impl IntoResponse {
    "OK"
}

// WebSocketハンドラ
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("New WebSocket connection established");
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // クライアントが切断された場合など
            return;
        };

        if socket.send(msg).await.is_err() {
            // 送信に失敗した場合
            return;
        }
    }
}

// プロキシ（ルーティング）用ハンドラ
async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    req: Request,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    let (_parts, body) = req.into_parts();

    // ボディの読み取り（1MB制限）
    let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            warn!("Failed to read request body: {}", e);
            return (axum::http::StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    // JSONとしてパースを試みる
    if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        // ツール呼び出しの検証（インターセプション）
        if let Err(msg) = state.interceptor.validate_tools(&json_body) {
            return (axum::http::StatusCode::FORBIDDEN, msg).into_response();
        }
    }

    // キャッシュの確認
    let cache_key = if state.caching_config.enabled {
        let key = CacheKey::new(&path, &bytes);
        if let Some(cached) = state.cache.get(&key).await {
            info!("Cache hit for path: {}", path);
            let mut res = cached.body.into_response();
            *res.status_mut() = axum::http::StatusCode::from_u16(cached.status).unwrap();
            for (k, v) in cached.headers {
                if let Ok(name) = axum::http::HeaderName::from_bytes(k.as_bytes()) {
                    if let Ok(value) = axum::http::HeaderValue::from_str(&v) {
                        res.headers_mut().insert(name, value);
                    }
                }
            }
            return res;
        }
        Some(key)
    } else {
        None
    };

    if let Some(rule) = state.router.resolve(&path) {
        info!("Matched rule: {} -> {} ({})", rule.path, rule.target_model, rule.target_url);
        
        let response_text = format!("Routing request to {} (Model: {})", rule.target_url, rule.target_model);
        
        // キャッシュの保存（非ストリーミングの場合の暫定的な実装）
        if let Some(key) = cache_key {
            let mut headers = std::collections::HashMap::new();
            headers.insert("content-type".to_string(), "text/plain; charset=utf-8".to_string());
            
            state.cache.set(key, CachedResponse {
                status: 200,
                headers,
                body: response_text.clone().into(),
            }).await;
        }

        response_text.into_response()
    } else {
        warn!("No route matched for path: {}", path);
        "No matching route found".into_response()
    }
}

// ストリーミングテスト用ハンドラ
async fn stream_test_handler(
    State(state): State<Arc<AppState>>,
    req: Request,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    
    // キャッシュの確認
    if state.caching_config.enabled {
        // テスト用なので固定の空ボディでハッシュ
        let key = CacheKey::new(&path, &[]);
        if let Some(cached) = state.cache.get(&key).await {
            info!("Cache hit (streaming) for path: {}", path);
            let mut res = cached.body.into_response();
            // SSEとして返すためのヘッダー設定
            res.headers_mut().insert(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("text/event-stream"));
            return res;
        }
    }

    info!("Stream test requested");

    let stream = stream::iter(vec![
        Ok::<&str, Infallible>(r#"{"choices":[{"delta":{"content":"Hello, "}}]}"#),
        Ok::<&str, Infallible>(r#"{"choices":[{"delta":{"content":"this "}}]}"#),
        Ok::<&str, Infallible>(r#"{"choices":[{"delta":{"content":"is "}}]}"#),
        Ok::<&str, Infallible>(r#"{"choices":[{"delta":{"content":"a "}}]}"#),
        Ok::<&str, Infallible>(r#"{"choices":[{"delta":{"content":"stream. "}}]}"#),
        // 途中からツール呼び出しをシミュレート
        Ok::<&str, Infallible>(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"get_weather"}}]}}]}"#),
        Ok::<&str, Infallible>(r#"{"choices":[{"delta":{"tool_calls":[{"index":1,"function":{"name":"rm_rf"}}]}}]}"#),
        Ok::<&str, Infallible>("[DONE]"),
    ])
    .throttle(Duration::from_millis(500));

    // StreamingAnalyzer でラップして検証を行う
    let bytes_stream = futures::StreamExt::map(stream, |res| {
        match res {
            Ok(data) => {
                let formatted = if data == "[DONE]" {
                    "data: [DONE]\n\n".to_string()
                } else {
                    format!("data: {}\n\n", data)
                };
                Ok::<Bytes, axum::Error>(Bytes::from(formatted))
            },
            Err(_) => unreachable!(),
        }
    });

    let cache_info = if state.caching_config.enabled {
        let key = CacheKey::new(&path, &[]);
        Some((state.cache.clone(), key))
    } else {
        None
    };

    let analyzer = StreamingAnalyzer::new(
        Box::pin(bytes_stream), 
        Arc::new(state.interceptor.clone()),
        cache_info,
    );
    
    Sse::new(analyzer)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}
