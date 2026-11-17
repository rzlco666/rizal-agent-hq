use axum::{Json, extract::State};
use hq_store::Store;
use serde_json::json;

#[derive(Clone)]
pub struct HealthState {
    pub store: Store,
}

pub async fn health(State(st): State<HealthState>) -> Json<serde_json::Value> {
    match st.store.health().await {
        Ok(h) => Json(json!({"ok": true, "last_seq": h.last_seq})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}
