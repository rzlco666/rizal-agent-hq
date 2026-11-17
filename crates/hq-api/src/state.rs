use crate::auth::AuthConfig;
use hq_store::{PatchRing, Store};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub ring: Arc<Mutex<PatchRing>>,
    pub auth: AuthConfig,
    pub started_at: Instant,
}
