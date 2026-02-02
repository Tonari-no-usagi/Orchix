use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::Response,
    middleware::Next,
    extract::State,
};
use std::sync::Arc;
use crate::networking::AppState;
use tracing::warn;

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // セキュリティ設定が空（APIキーが1つも設定されていない）の場合は認証をスキップ（開発用）
    if state.security.api_keys.is_empty() {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => {
            let key = &auth[7..];
            if state.security.api_keys.iter().any(|k| k == key) {
                Ok(next.run(req).await)
            } else {
                warn!("Invalid API key attempt");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            warn!("Missing or invalid Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
