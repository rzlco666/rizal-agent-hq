//! Axum router, REST v1, WS/SSE, webhook ingress, OpenAPI, and auth.

#![forbid(unsafe_code)]

pub mod auth;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod health;
pub mod openapi;
pub mod router;
pub mod sse;
pub mod state;
pub mod static_assets;
pub mod ws;

pub use router::{api_router, build_router};
pub use state::AppState;
