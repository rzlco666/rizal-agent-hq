#![allow(unused_imports, dead_code)]
use hq_api::api_router;
use hq_core::*;
use hq_store::*;
use std::net::SocketAddr;
use tempfile::TempDir;
use tokio::net::TcpListener;

async fn spawn_test_api() -> (SocketAddr, Ingestor, TempDir) {
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

async fn next_json(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    use futures::StreamExt;
    while let Some(Ok(msg)) = ws.next().await {
        if let tokio_tungstenite::tungstenite::Message::Text(t) = msg {
            return serde_json::from_str(&t).unwrap();
        }
    }
    panic!("no json");
}

#[tokio::test]
async fn ws_sends_hello_then_patch() {
    let (addr, ing, _dir) = spawn_test_api().await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    let hello = next_json(&mut ws).await;
    assert_eq!(hello["type"], "hello");
    ing.ingest(vec![agent_observed("agent:omp", "bekerja")])
        .await
        .unwrap();
    let msg = next_json(&mut ws).await;
    assert_eq!(msg["type"], "patch");
    assert!(msg["seq"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn ws_reconnect_replays_missed_patches() {
    let (addr, ing, _dir) = spawn_test_api().await;
    ing.ingest(vec![heartbeat("agent:muse", "a")])
        .await
        .unwrap();
    let seq = ing.store().last_seq().await.unwrap();
    ing.ingest(vec![heartbeat("agent:muse", "b")])
        .await
        .unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since={}", seq.0))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");
    assert_eq!(next_json(&mut ws).await["type"], "patch");
}

#[tokio::test]
async fn stale_since_asks_for_snapshot() {
    let (addr, _ing, _dir) = spawn_test_api().await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=1"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "snapshot_required");
}

#[tokio::test]
async fn ws_agent_disappeared_is_an_event_delta() {
    let (addr, ing, _dir) = spawn_test_api().await;
    let mut draft = agent_observed("agent:omp", "bekerja");
    draft.kind = EventKind::AgentDisappeared;
    draft.payload = serde_json::json!({ "disappeared_at_ms": 1000 });
    ing.ingest(vec![draft]).await.unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");
    let patch = next_json(&mut ws).await;
    assert_eq!(patch["upserts"][0]["type"], "agent");
    assert_eq!(patch["upserts"][0]["event_kind"], "agent_disappeared");
    assert_eq!(patch["upserts"][0]["payload"]["disappeared_at_ms"], 1000);
}

#[tokio::test]
async fn ws_alert_and_usage_upserts_include_type_and_payload() {
    let (addr, ing, _dir) = spawn_test_api().await;
    ing.ingest(vec![
        draft(
            EventKind::AlertRaised,
            Subject::Alert(AlertId("alert-1".to_string())),
            serde_json::json!({"message":"needs review"}),
        ),
        draft(
            EventKind::UsageRecorded,
            Subject::Usage(UsageScope::Global),
            serde_json::json!({"tokens":42}),
        ),
    ])
    .await
    .unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");
    let alert = next_json(&mut ws).await["upserts"][0].clone();
    assert_eq!(alert["type"], "alert");
    assert_eq!(alert["event_kind"], "alert_raised");
    assert_eq!(alert["payload"]["message"], "needs review");
    let usage = next_json(&mut ws).await["upserts"][0].clone();
    assert_eq!(usage["type"], "usage");
    assert_eq!(usage["event_kind"], "usage_recorded");
    assert_eq!(usage["payload"]["tokens"], 42);
}

#[tokio::test]
async fn ws_session_delta_targets_stable_session_subject() {
    let (addr, ing, _dir) = spawn_test_api().await;
    ing.ingest(vec![draft(
        EventKind::SessionUpdated,
        Subject::Session(SessionId("agent:omp/session/stable-1".to_string())),
        serde_json::json!({"updated_at_ms":1000}),
    )])
    .await
    .unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");
    let delta = next_json(&mut ws).await["upserts"][0].clone();
    assert_eq!(delta["type"], "task");
    assert_eq!(delta["id"], "agent:omp/session/stable-1");
    assert_eq!(delta["event_kind"], "session_updated");
}

#[tokio::test]
async fn ws_session_usage_delta_targets_session_subject() {
    let (addr, ing, _dir) = spawn_test_api().await;
    ing.ingest(vec![draft(
        EventKind::UsageRecorded,
        Subject::Usage(UsageScope::Session(SessionId(
            "agent:omp/session/stable-2".to_string(),
        ))),
        serde_json::json!({"tokens":42}),
    )])
    .await
    .unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");
    let delta = next_json(&mut ws).await["upserts"][0].clone();
    assert_eq!(delta["type"], "usage");
    assert_eq!(delta["id"], "agent:omp/session/stable-2");
}

#[tokio::test]
async fn ws_agent_log_is_a_redacted_log_delta_not_a_task() {
    let (addr, ing, _dir) = spawn_test_api().await;
    ing.ingest(vec![draft(
        EventKind::LogRecorded,
        Subject::Agent(AgentId("agent:omp".to_string())),
        serde_json::json!({"message":"token ghp_abcdefghijklmnop"}),
    )])
    .await
    .unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");
    let patch = next_json(&mut ws).await;
    let delta = &patch["upserts"][0];
    assert_eq!(delta["type"], "log");
    assert_eq!(delta["id"], "agent:omp");
    assert_eq!(delta["event_kind"], "log_recorded");
    assert!(!patch.to_string().contains("ghp_abcdefghijklmnop"));
}

#[tokio::test]
async fn ws_alert_lifecycle_deltas_keep_event_kind() {
    let (addr, ing, _dir) = spawn_test_api().await;
    let target = Subject::Alert(AlertId("alert-lifecycle".to_string()));
    ing.ingest(vec![
        draft(
            EventKind::AlertAcknowledged,
            target.clone(),
            serde_json::json!({"at_ms":1000}),
        ),
        draft(
            EventKind::AlertResolved,
            target,
            serde_json::json!({"at_ms":2000}),
        ),
    ])
    .await
    .unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");
    let acknowledged = next_json(&mut ws).await["upserts"][0].clone();
    let resolved = next_json(&mut ws).await["upserts"][0].clone();
    assert_eq!(acknowledged["id"], "alert-lifecycle");
    assert_eq!(acknowledged["event_kind"], "alert_acknowledged");
    assert_eq!(resolved["event_kind"], "alert_resolved");
}

fn draft(kind: EventKind, subject: Subject, payload: serde_json::Value) -> EventDraft {
    EventDraft {
        event_ts_ms: 1000,
        kind,
        subject,
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload,
    }
}

#[tokio::test]
async fn ws_read_error_requests_snapshot() {
    let (addr, ing, _dir) = spawn_test_api().await;
    ing.store().test_fail_reads(true);
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "snapshot_required");
}

#[tokio::test]
async fn ws_last_seq_error_requests_snapshot() {
    let (addr, ing, _dir) = spawn_test_api().await;
    ing.store().test_fail_last_seq(true);
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "snapshot_required");
}
#[tokio::test]
async fn ws_lagged_recovery_requires_snapshot_when_ring_has_partial_history() {
    use futures::StreamExt;

    let (addr, ing, _dir) = spawn_test_api().await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");

    let payload = "x".repeat(2048);
    ing.ingest(
        (0..2049)
            .map(|i| {
                draft(
                    EventKind::AlertRaised,
                    Subject::Alert(AlertId(format!("alert-{i}"))),
                    serde_json::json!({"message": payload}),
                )
            })
            .collect(),
    )
    .await
    .unwrap();

    let snapshot = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) = ws.next().await {
            let message: serde_json::Value = serde_json::from_str(&text).unwrap();
            if message["type"] == "snapshot_required" {
                return Some(message);
            }
        }
        None
    })
    .await
    .expect("WS did not produce a recovery response");
    assert!(
        snapshot.is_some(),
        "WS ended or recovered without a snapshot"
    );
}

#[tokio::test]
async fn ws_max_since_cursor_requires_snapshot() {
    let (addr, _ing, _dir) = spawn_test_api().await;
    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since={}", i64::MAX))
            .await
            .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "snapshot_required");
}

#[tokio::test]
async fn ws_client_disconnect_ends_server_connection() {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let (addr, _ing, _dir) = spawn_test_api().await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?since=0"))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "hello");

    ws.send(Message::Close(None)).await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
        .await
        .expect("server did not finish the WebSocket close handshake");
    assert!(
        response.is_none() || matches!(response, Some(Ok(Message::Close(_)))),
        "unexpected disconnect response: {response:?}"
    );
}

// helpers
fn agent_observed(agent: &str, status: &str) -> EventDraft {
    let family = agent.strip_prefix("agent:").unwrap_or(agent);
    EventDraft {
        event_ts_ms: 1000,
        kind: EventKind::AgentObserved,
        subject: Subject::Agent(AgentId(agent.to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: serde_json::json!({
            "observed_at_ms": 1000,
            "runtime_key": format!("presence:{family}"),
            "status": status,
            "kind": "Utama",
            "activity": { "key": "agent.process_running", "params": { "family": family } },
            "processes": 1,
            "cpu_percent": 0.0,
            "evidence": "L1"
        }),
    }
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
