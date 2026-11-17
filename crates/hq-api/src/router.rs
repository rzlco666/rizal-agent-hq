use crate::auth::{self, AuthConfig};
use crate::health::{health, health_legacy};
use crate::sse;
use crate::state::AppState;
use crate::ws;
use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use hq_store::{PatchRing, Store};

use std::sync::{Arc, Mutex};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

fn spawn_patch_forwarder(
    mut rx: tokio::sync::broadcast::Receiver<hq_store::Patch>,
    ring: Arc<Mutex<PatchRing>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(patch) => {
                    if let Ok(mut ring) = ring.lock() {
                        ring.push(patch);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

/// Fill `request_id` in error envelopes from the request-id header.
async fn fill_request_id(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let response = next.run(request).await;
    let Some(request_id) = request_id else {
        return response;
    };
    if !response.status().is_client_error() && !response.status().is_server_error() {
        return response;
    }
    let (parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, 64 * 1024)
        .await
        .unwrap_or_default();
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Response::from_parts(parts, Body::from(bytes));
    };
    if value.get("code").is_none() || value.get("request_id").is_none() {
        return Response::from_parts(parts, Body::from(bytes));
    }
    value["request_id"] = serde_json::json!(request_id);
    Response::from_parts(parts, Json(value).into_response().into_body())
}

pub fn build_router(state: AppState) -> Router {
    drop(spawn_patch_forwarder(
        state.store.patches(),
        state.ring.clone(),
    ));

    Router::new()
        .route("/health", get(health_legacy))
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/pair", axum::routing::post(auth::pair))
        .route(
            "/api/v1/auth/ws-ticket",
            axum::routing::post(auth::ws_ticket),
        )
        .route("/api/v1/devices", get(auth::devices))
        .route(
            "/api/v1/devices/{id}",
            axum::routing::delete(auth::revoke_device),
        )
        .route("/ws", get(ws::ws_handler))
        .route("/api/v1/overview", get(crate::handlers::overview::overview))
        .route("/api/v1/agents", get(crate::handlers::agents::agents))
        .route(
            "/api/v1/agents/{id}",
            get(crate::handlers::agents::agent_detail),
        )
        .route(
            "/api/v1/agents/{id}/tasks",
            get(crate::handlers::agents::agent_tasks),
        )
        .route(
            "/api/v1/agents/{id}/logs",
            get(crate::handlers::agents::agent_logs),
        )
        .route(
            "/api/v1/agents/{id}/sessions",
            get(crate::handlers::agents::agent_sessions),
        )
        .route(
            "/api/v1/agents/{id}/files",
            get(crate::handlers::agents::agent_files),
        )
        .route(
            "/api/v1/agents/{id}/usage",
            get(crate::handlers::agents::agent_usage),
        )
        .route("/api/v1/tasks", get(crate::handlers::tasks::tasks))
        .route(
            "/api/v1/tasks/{id}",
            get(crate::handlers::tasks::task_detail),
        )
        .route("/api/v1/alerts", get(crate::handlers::alerts::alerts))
        .route(
            "/api/v1/alerts/{id}/ack",
            axum::routing::post(crate::handlers::alerts::ack_alert),
        )
        .route(
            "/api/v1/alerts/{id}/snooze",
            axum::routing::post(crate::handlers::alerts::snooze_alert),
        )
        .route(
            "/api/v1/alerts/{id}/resolve",
            axum::routing::post(crate::handlers::alerts::resolve_alert),
        )
        .route(
            "/api/v1/alerts/{id}",
            get(crate::handlers::alerts::alert_detail),
        )
        .route("/api/v1/timeline", get(crate::handlers::timeline::timeline))
        .route("/api/v1/events", get(crate::handlers::timeline::timeline))
        .route("/api/v1/usage", get(crate::handlers::usage::usage))
        .route("/api/v1/costs", get(crate::handlers::usage::costs))
        .route(
            "/api/v1/openapi.json",
            get(crate::handlers::meta::openapi_json),
        )
        .route("/api/v1/events/stream", get(sse::sse_handler))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state, auth::auth_middleware))
        .layer(middleware::from_fn(fill_request_id))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CatchPanicLayer::new())
}

/// Compatibility constructor used by S03 WS/SSE tests.
pub fn api_router(store: Store, ring: PatchRing) -> Router {
    build_router(AppState {
        store,
        ring: Arc::new(Mutex::new(ring)),
        auth: AuthConfig::default(),
        started_at: std::time::Instant::now(),
        tickets: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::spawn_patch_forwarder;
    use hq_store::{Patch, PatchRing, Seq};
    use std::sync::{Arc, Mutex};
    use tokio::sync::broadcast;

    fn patch(seq: i64) -> Patch {
        Patch {
            seq: Seq(seq),
            upserts: Vec::new(),
            deletes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn forwarder_resumes_after_lag_and_pushes_later_patch() {
        let (tx, rx) = broadcast::channel(1);
        let ring = Arc::new(Mutex::new(PatchRing::new(2)));
        tx.send(patch(1)).unwrap();
        tx.send(patch(2)).unwrap();

        let forwarder = spawn_patch_forwarder(rx, ring.clone());
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if ring.lock().unwrap().since(Seq(2)).is_some() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("forwarder did not resume after lag");

        tx.send(patch(3)).unwrap();
        drop(tx);
        forwarder.await.unwrap();

        let ring = ring.lock().unwrap();
        assert!(ring.since(Seq(1)).is_none());
        let later = ring.since(Seq(2)).unwrap();
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].seq, Seq(3));
    }
}
