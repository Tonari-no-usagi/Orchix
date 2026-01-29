use tracing::{info, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use std::str::FromStr;

mod networking;
mod config;
mod routing;
mod interception;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 設定の読み込み
    let app_config = config::AppConfig::load()?;

    // ロギングの初期化
    let filter = EnvFilter::from_str(&app_config.log.level)
        .unwrap_or_else(|_| EnvFilter::new("info"));
        
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Orchix Agentic Proxy...");
    info!("Configuration loaded: {:?}", app_config);

    // Networkingサーバーの起動
    networking::run_server(app_config.server, app_config.routing, app_config.interception).await?;

    Ok(())
}
