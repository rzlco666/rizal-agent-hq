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

// helpers
fn agent_observed(agent: &str, status: &str) -> EventDraft {
    EventDraft {
        event_ts_ms: 1000,
        kind: EventKind::AgentObserved,
        subject: Subject::Agent(AgentId(agent.to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: serde_json::json!({ "observed_at_ms": 1000, "status": status }),
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
