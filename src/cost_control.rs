use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::config::CostConfig;
use tracing::{info, warn};

pub struct CostManager {
    config: CostConfig,
    // クライアントIDごとのリクエスト履歴（秒単位のタイムスタンプ）
    request_history: Arc<Mutex<HashMap<String, Vec<u64>>>>,
    // クライアントIDごとの累積トークン数
    usage_stats: Arc<Mutex<HashMap<String, u32>>>,
}

impl CostManager {
    pub fn new(config: CostConfig) -> Self {
        Self {
            config,
            request_history: Arc::new(Mutex::new(HashMap::new())),
            usage_stats: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// トークン数を簡易推定する（文字数 / 4）
    pub fn estimate_tokens(&self, text: &str) -> u32 {
        // 日本語などのマルチバイト文字も考慮し、文字数ベースで計算
        let char_count = text.chars().count() as u32;
        // 1トークンを平均4文字と仮定（簡易版）
        (char_count / 4).max(1)
    }

    /// レート制限のチェック（直近1時間の回数）
    pub async fn check_rate_limit(&self, client_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let mut status = self.request_history.lock().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let history = status.entry(client_id.to_string()).or_insert(Vec::new());
        
        // 1時間以上前の履歴を削除
        history.retain(|&t| t > now - 3600);

        if history.len() >= self.config.hourly_rate_limit as usize {
            warn!("Rate limit exceeded for client: {}", client_id);
            return false;
        }

        // 今回のリクエストを記録
        history.push(now);
        true
    }

    /// 予算（累積トークン）のチェック
    pub async fn check_budget(&self, client_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let stats = self.usage_stats.lock().await;
        let current_usage = stats.get(client_id).cloned().unwrap_or(0);

        if current_usage >= self.config.daily_budget_tokens {
            warn!("Budget exceeded for client: {}. Current: {}, limit: {}", client_id, current_usage, self.config.daily_budget_tokens);
            return false;
        }

        true
    }

    /// 使用量の記録
    pub async fn track_usage(&self, client_id: &str, tokens: u32) {
        if !self.config.enabled {
            return;
        }

        let mut stats = self.usage_stats.lock().await;
        let usage = stats.entry(client_id.to_string()).or_insert(0);
        *usage += tokens;
        
        info!("Usage tracked for {}: +{} tokens (Total: {})", client_id, tokens, *usage);
    }

    /// 最大リクエストトークン数のチェック
    pub fn is_within_max_tokens(&self, tokens: u32) -> bool {
        if !self.config.enabled {
            return true;
        }
        tokens <= self.config.max_request_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CostConfig;

    fn test_config() -> CostConfig {
        CostConfig {
            enabled: true,
            hourly_rate_limit: 2,
            daily_budget_tokens: 100,
            max_request_tokens: 50,
        }
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let manager = CostManager::new(test_config());
        let client = "test_user";

        assert!(manager.check_rate_limit(client).await);
        assert!(manager.check_rate_limit(client).await);
        assert!(!manager.check_rate_limit(client).await); // 3回目は制限
    }

    #[tokio::test]
    async fn test_budgeting() {
        let manager = CostManager::new(test_config());
        let client = "test_user";

        assert!(manager.check_budget(client).await);
        manager.track_usage(client, 101).await;
        assert!(!manager.check_budget(client).await); // 予算超過
    }

    #[test]
    fn test_token_estimation() {
        let manager = CostManager::new(test_config());
        assert_eq!(manager.estimate_tokens("12345678"), 2);
        assert_eq!(manager.estimate_tokens("あいうえ"), 1);
    }
}
