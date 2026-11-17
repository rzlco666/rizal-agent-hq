use axum::{extract::Request, middleware::Next, response::Response};

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub trust_loopback: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            trust_loopback: std::env::var_os("HQ_TRUST_LOOPBACK")
                .is_none_or(|v| v != "0"),
        }
    }
}

pub async fn auth_middleware(_request: Request, next: Next) -> Response {
    next.run(_request).await
}
