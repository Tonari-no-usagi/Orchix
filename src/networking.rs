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

pub async fn run_server(config: ServerConfig, rules: Vec<RouteRule>) -> anyhow::Result<()> {
    // ルーター（Orchix側）の初期化
    let orchix_router = Arc::new(OrchixRouter::new(rules));

    // HTTPルーター（Axum側）の設定
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
        // すべてのパスを一旦受け入れ、内部ルーターで処理
        .fallback(any(proxy_handler))
        .with_state(orchix_router);

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
    State(router): State<Arc<OrchixRouter>>,
    req: Request,
) -> impl IntoResponse {
    let path = req.uri().path();
    
    if let Some(rule) = router.resolve(path) {
        info!("Matched rule: {} -> {} ({})", rule.path, rule.target_model, rule.target_url);
        format!("Routing request to {} (Model: {})", rule.target_url, rule.target_model).into_response()
    } else {
        warn!("No route matched for path: {}", path);
        "No matching route found".into_response()
    }
}
