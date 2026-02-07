use sha2::{Sha256, Digest};
use moka::future::Cache;
use std::time::Duration;
use bytes::Bytes;
use serde::{Serialize, Deserialize};
use crate::config::CacheConfig;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(pub String);

impl CacheKey {
    pub fn new(path: &str, body: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(path.as_bytes());
        hasher.update(body);
        let result = hasher.finalize();
        Self(hex::encode(result))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Bytes,
}

#[derive(Clone)]
pub struct OrchixCache {
    client: Cache<CacheKey, CachedResponse>,
}

impl OrchixCache {
    pub fn new(config: &CacheConfig) -> Self {
        let client = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .build();
        
        Self { client }
    }

    pub async fn get(&self, key: &CacheKey) -> Option<CachedResponse> {
        self.client.get(key).await
    }

    pub async fn set(&self, key: CacheKey, response: CachedResponse) {
        self.client.insert(key, response).await;
    }
}
