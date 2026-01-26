use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod networking;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ロギングの初期化
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Orchix Agentic Proxy...");

    // Networkingサーバーの起動
    networking::run_server().await?;

    Ok(())
}
