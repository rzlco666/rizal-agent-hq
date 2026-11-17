//! Axum router, REST v1, WS/SSE, webhook ingress, OpenAPI, and auth.

#![forbid(unsafe_code)]

pub mod auth;
pub mod error;
pub mod health;
pub mod router;
pub mod state;
pub mod sse;
pub mod ws;

pub use router::{api_router, build_router};
pub use state::AppState;
