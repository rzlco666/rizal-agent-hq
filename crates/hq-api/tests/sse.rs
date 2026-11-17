use axum::{body::Body, extract::ConnectInfo, http::Request};
use hq_api::api_router;
use hq_core::*;
use hq_store::*;
use std::net::SocketAddr;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tower::ServiceExt;

async fn spawn() -> (SocketAddr, Ingestor, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let ring = PatchRing::new(1000);
    let ing = Ingestor::new(store.clone());
    let app = api_router(store, ring);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    (addr, ing, dir)
}

async fn request(addr: SocketAddr, headers: &[(&str, &str)], marker: &str) -> String {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut request = String::from("GET /api/v1/events/stream HTTP/1.1\r\nHost: localhost\r\n");
        for (name, value) in headers {
            request.push_str(name);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        let mut buffer = [0; 4096];
        loop {
            let n = stream.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            response.push_str(&String::from_utf8_lossy(&buffer[..n]));
            if response.contains(marker) {
                break;
            }
        }
        response
    })
    .await
    .expect("SSE response did not contain the expected marker before timeout")
}

#[tokio::test]
async fn sse_returns_event_stream() {
    let (addr, _ing, _dir) = spawn().await;
    let response = request(addr, &[], "\r\n\r\n").await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        response
            .to_ascii_lowercase()
            .contains("content-type: text/event-stream"),
        "{response}"
    );
}

#[tokio::test]
async fn sse_rejects_forwarded_peer() {
    let (addr, _ing, _dir) = spawn().await;
    let response = request(addr, &[("X-Forwarded-For", "127.0.0.1")], "\r\n\r\n").await;
    assert!(response.starts_with("HTTP/1.1 401"), "{response}");
}

#[tokio::test]
async fn last_event_id_replays_stored_event() {
    let (addr, ing, _dir) = spawn().await;
    let event = ing
        .ingest(vec![heartbeat("agent:muse", "one")])
        .await
        .unwrap();
    let marker = format!("id: {}", event.accepted[0].seq.0);
    let response = request(
        addr,
        &[("Last-Event-ID", &(event.accepted[0].seq.0 - 1).to_string())],
        &marker,
    )
    .await;
    assert!(
        response.contains(&format!("id: {}", event.accepted[0].seq.0)),
        "{response}"
    );
    assert!(response.contains("\"type\":\"agent\""), "{response}");
}

#[tokio::test]
async fn exactly_1000_events_replay_without_snapshot() {
    let (addr, ing, _dir) = spawn().await;
    let drafts = (0..1000)
        .map(|i| heartbeat("agent:muse", &format!("event-{i}")))
        .collect();
    ing.ingest(drafts).await.unwrap();
    let response = request(addr, &[], "id: 1000").await;
    assert!(!response.contains("snapshot_required"), "{response}");
    assert!(response.contains("id: 1"), "{response}");
}

#[tokio::test]
async fn more_than_1000_events_requires_snapshot() {
    let (addr, ing, _dir) = spawn().await;
    let drafts = (0..1001)
        .map(|i| heartbeat("agent:muse", &format!("event-{i}")))
        .collect();
    ing.ingest(drafts).await.unwrap();
    let response = request(addr, &[], "snapshot_required").await;
    assert!(response.contains("snapshot_required"), "{response}");
}

#[tokio::test]
async fn replay_error_returns_snapshot_required_event() {
    let (addr, ing, _dir) = spawn().await;
    ing.store().test_fail_reads(true);
    let response = request(addr, &[], "snapshot_required").await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        response.contains("\"type\":\"snapshot_required\""),
        "{response}"
    );
}

#[tokio::test]
async fn last_seq_error_returns_snapshot_required_event() {
    let (addr, ing, _dir) = spawn().await;
    ing.store().test_fail_last_seq(true);
    let response = request(addr, &[], "snapshot_required").await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        response.contains("\"type\":\"snapshot_required\""),
        "{response}"
    );
}

#[tokio::test]
async fn sse_entity_upserts_include_type_and_redacted_payload() {
    let (addr, ing, _dir) = spawn().await;
    ing.ingest(vec![EventDraft {
        event_ts_ms: 1000,
        kind: EventKind::AlertRaised,
        subject: Subject::Alert(AlertId("alert-1".to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: serde_json::json!({"message":"needs review"}),
    }])
    .await
    .unwrap();
    let response = request(
        addr,
        &[("Last-Event-ID", "0")],
        r#""payload":{"message":"needs review"}"#,
    )
    .await;
    assert!(response.contains("\"type\":\"alert\""), "{response}");
    assert!(
        response.contains("\"event_kind\":\"alert_raised\""),
        "{response}"
    );
    assert!(
        response.contains("\"payload\":{\"message\":\"needs review\"}"),
        "{response}"
    );
}

#[tokio::test]
async fn future_last_event_id_uses_observed_snapshot_watermark() {
    let (addr, _ing, _dir) = spawn().await;
    let response = request(addr, &[("Last-Event-ID", "99")], "snapshot_required").await;
    assert!(
        response.contains("\"type\":\"snapshot_required\""),
        "{response}"
    );
    assert!(response.contains("\"seq\":0"), "{response}");
}
#[tokio::test]
async fn sse_live_broadcast_lag_emits_snapshot_required() {
    use futures::StreamExt;

    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let ing = Ingestor::new(store.clone());
    let app = api_router(store, PatchRing::new(1000));
    let request = Request::builder()
        .uri("/api/v1/events/stream")
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1234))))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    ing.ingest(
        (0..1025)
            .map(|i| heartbeat("agent:muse", &format!("lag-{i}")))
            .collect(),
    )
    .await
    .unwrap();

    let mut body = response.into_body().into_data_stream();
    let mut frames = String::new();
    while let Some(Ok(frame)) = body.next().await {
        frames.push_str(&String::from_utf8_lossy(&frame));
        if frames.contains("\"type\":\"snapshot_required\"") {
            break;
        }
    }
    assert!(
        frames.contains("\"type\":\"snapshot_required\""),
        "{frames}"
    );
    assert!(frames.contains("\"seq\":1025"), "{frames}");
}

fn heartbeat(agent: &str, key: &str) -> EventDraft {
    EventDraft {
        event_ts_ms: 1000,
        kind: EventKind::AgentSeen,
        subject: Subject::Agent(AgentId(agent.to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(key.to_string()),
        payload: serde_json::json!({ "seen_at_ms": 1000 }),
    }
}
#[tokio::test]
async fn sse_rejects_non_loopback_peer() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let app = api_router(store, PatchRing::new(1000));
    let request = Request::builder()
        .uri("/api/v1/events/stream")
        .extension(ConnectInfo(SocketAddr::from(([192, 0, 2, 1], 1234))))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}
