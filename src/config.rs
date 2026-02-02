use serde::Deserialize;
use config::{Config, ConfigError, File, Environment};
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub log: LogConfig,
    pub routing: Vec<crate::routing::RouteRule>,
    pub interception: crate::interception::InterceptionConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub api_keys: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // .envファイルの読み込み（存在しなくても無視）
        dotenvy::dotenv().ok();

        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // デフォルト値の設定
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3000)?
            .set_default("log.level", "info")?
            .set_default("security.api_keys", Vec::<String>::new())?
            // 設定ファイル (config.toml) の読み込み
            .add_source(File::with_name("config").required(false))
            // 環境に応じた設定ファイル (config/development.toml など) の読み込み
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // 環境変数の読み込み (ORCHIX_SERVER__PORT=4000 など)
            .add_source(Environment::with_prefix("ORCHIX").separator("__"))
            .build()?;

        s.try_deserialize()
    }
}
