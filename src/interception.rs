use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};

#[derive(Debug, Deserialize, Clone)]
pub struct InterceptionConfig {
    pub forbidden_tools: Vec<String>,
}

pub struct Interceptor {
    pub config: InterceptionConfig,
}

impl Interceptor {
    pub fn new(config: InterceptionConfig) -> Self {
        Self { config }
    }

    /// リクエストボディ内のツール呼び出しを検証します
    pub fn validate_tools(&self, body: &Value) -> Result<(), String> {
        info!("Intercepting tool calls in request body...");

        // OpenAI 互換の tool_calls 構造を想定
        if let Some(tool_calls) = body.get("tool_calls").and_then(|v| v.as_array()) {
            for call in tool_calls {
                if let Some(name) = call.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()) {
                    if self.config.forbidden_tools.contains(&name.to_string()) {
                        warn!("Forbidden tool call detected: {}", name);
                        return Err(format!("Tool '{}' is blocked by Orchix security policy", name));
                    }
                }
            }
        }

        // 古い functions API の場合
        if let Some(name) = body.get("function_call").and_then(|f| f.get("name")).and_then(|n| n.as_str()) {
            if self.config.forbidden_tools.contains(&name.to_string()) {
                warn!("Forbidden function call detected: {}", name);
                return Err(format!("Function '{}' is blocked by Orchix security policy", name));
            }
        }

        Ok(())
    }
}
