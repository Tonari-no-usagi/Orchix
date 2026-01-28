use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize, Clone)]
pub struct RouteRule {
    pub path: String,
    pub target_model: String,
    pub target_url: String,
}

pub struct Router {
    pub rules: Vec<RouteRule>,
}

impl Router {
    pub fn new(rules: Vec<RouteRule>) -> Self {
        Self { rules }
    }

    pub fn resolve(&self, path: &str) -> Option<&RouteRule> {
        info!("Resolving route for path: {}", path);
        // シンプルな前方一致でのマッチング
        self.rules.iter().find(|rule| path.starts_with(&rule.path))
    }
}
