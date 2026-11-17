use crate::health::{HealthState, health};
use crate::sse;
use crate::ws;
use axum::{Router, routing::get};
use hq_store::{PatchRing, Store};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub ring: Arc<Mutex<PatchRing>>,
}

pub fn api_router(store: Store, ring: PatchRing) -> Router {
    let state = AppState {
        store: store.clone(),
        ring: Arc::new(Mutex::new(ring)),
    };
    // forward broadcast patches into ring
    let fwd_store = store.clone();
    let fwd_ring = state.ring.clone();
    tokio::spawn(async move {
        let mut rx = fwd_store.patches();
        while let Ok(p) = rx.recv().await {
            if let Ok(mut g) = fwd_ring.lock() {
                g.push(p);
            }
        }
    });

    let health_state = HealthState { store };

    Router::new()
        .route("/health", get(health).with_state(health_state))
        .route("/ws", get(ws::ws_handler))
        .route("/api/v1/events/stream", get(sse::sse_handler))
        .with_state(state)
}
