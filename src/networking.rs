use axum::{
    routing::get,
    response::IntoResponse,
    Router,
    extract::ws::{WebSocketUpgrade, WebSocket},
};
use std::net::SocketAddr;
use tracing::info;

pub async fn run_server() -> anyhow::Result<()> {
    // ルーターの設定
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler));

    // アドレスの設定
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
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
