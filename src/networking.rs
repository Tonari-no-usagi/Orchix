use axum::{
    routing::{get, any},
    response::IntoResponse,
    Router,
    extract::{ws::{WebSocketUpgrade, WebSocket}, State, Request},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn};
use crate::config::ServerConfig;
use crate::routing::{RouteRule, Router as OrchixRouter};
use crate::interception::{InterceptionConfig, Interceptor};
use crate::streaming::StreamingAnalyzer;
use futures::stream;
use axum::response::sse::Sse;
use std::convert::Infallible;
use tokio_stream::StreamExt as _;
use std::time::Duration;
use bytes::Bytes;

pub struct AppState {
    pub router: OrchixRouter,
    pub interceptor: Interceptor,
}

pub async fn run_server(
    config: ServerConfig, 
    rules: Vec<RouteRule>,
    interception_config: InterceptionConfig,
) -> anyhow::Result<()> {
    // 状態の初期化
    let state = Arc::new(AppState {
        router: OrchixRouter::new(rules),
        interceptor: Interceptor::new(interception_config),
    });

    // HTTPルーター（Axum側）の設定
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
        .route("/v1/stream_test", get(stream_test_handler))
        // すべてのパスを一旦受け入れ、内部ルーターで処理
        .fallback(any(proxy_handler))
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

    if let Some(rule) = state.router.resolve(&path) {
        info!("Matched rule: {} -> {} ({})", rule.path, rule.target_model, rule.target_url);
        format!("Routing request to {} (Model: {})", rule.target_url, rule.target_model).into_response()
    } else {
        warn!("No route matched for path: {}", path);
        "No matching route found".into_response()
    }
}

// ストリーミングテスト用ハンドラ
async fn stream_test_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
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

    let analyzer = StreamingAnalyzer::new(Box::pin(bytes_stream), Arc::new(state.interceptor.clone()));
    
    Sse::new(analyzer)
        .keep_alive(axum::response::sse::KeepAlive::default())
}
