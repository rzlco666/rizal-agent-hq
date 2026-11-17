#![allow(dead_code)]

use axum::{Router, body::Body, extract::ConnectInfo, http::Request, response::Response};
use hq_api::api_router;
use hq_core::*;
use hq_store::*;
use std::net::SocketAddr;
use tempfile::TempDir;
use tower::ServiceExt;

pub struct TestApp {
    pub app: Router,
    pub store: Store,
    pub ingestor: Ingestor,
    pub _dir: TempDir,
}

pub async fn test_app() -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let ingestor = Ingestor::new(store.clone());
    let app = api_router(store.clone(), PatchRing::new(1000));
    TestApp {
        app,
        store,
        ingestor,
        _dir: dir,
    }
}

pub fn loopback() -> SocketAddr {
    "127.0.0.1:45671".parse().unwrap()
}

pub fn remote() -> SocketAddr {
    "203.0.113.9:45671".parse().unwrap()
}

pub fn request(uri: &str, peer: SocketAddr) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .extension(ConnectInfo(peer))
        .body(Body::empty())
        .unwrap()
}

pub async fn get(app: Router, uri: &str, peer: SocketAddr) -> Response {
    app.oneshot(request(uri, peer)).await.unwrap()
}

pub async fn body_json(res: Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

pub fn draft(kind: EventKind, subject: Subject, payload: serde_json::Value) -> EventDraft {
    EventDraft {
        event_ts_ms: 1_700_000_000_000,
        kind,
        subject,
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload,
    }
}

pub fn agent_observed(id: &str, status: &str) -> EventDraft {
    draft(
        EventKind::AgentObserved,
        Subject::Agent(AgentId(id.to_string())),
        serde_json::json!({
            "name": id,
            "role": "agent",
            "kind": "utama",
            "runtime_key": id,
            "status": status,
            "evidence": "L1"
        }),
    )
}

pub fn alert_raised(id: &str) -> EventDraft {
    draft(
        EventKind::AlertRaised,
        Subject::Alert(AlertId(id.to_string())),
        serde_json::json!({
            "id": id,
            "fingerprint": format!("fp-{id}"),
            "rule_id": "rule-1",
            "level": "warning",
            "message": "perlu perhatian"
        }),
    )
}
