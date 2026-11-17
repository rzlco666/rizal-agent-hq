use hq_core::*;
use hq_store::*;
use tempfile::TempDir;

#[allow(dead_code)]
pub async fn test_store() -> (Store, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    (store, dir)
}

#[allow(dead_code)]
pub async fn seed_events(store: &Store) {
    store
        .append(vec![agent_observed("agent:omp", "bekerja")])
        .await
        .unwrap();
    store
        .append(vec![usage("agent:omp", 1_000, 1_700_000_000_000)])
        .await
        .unwrap();
}

#[allow(dead_code)]
pub async fn snapshot_json(store: &Store) -> String {
    let n = store.count_events().await.unwrap();
    let agents = store.agents().await.unwrap();
    format!("events={}|agents={:?}", n, agents)
}

#[allow(dead_code)]
pub fn agent_observed(agent: &str, status: &str) -> EventDraft {
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
            "runtime_key": "test",
            "status": status,
            "kind": "Utama",
            "activity": { "key": "agent.process_running", "params": { "family": "test" } },
            "processes": 1,
            "cpu_percent": 0.0,
            "evidence": "L1"
        }),
    }
}

#[allow(dead_code)]
pub fn heartbeat(agent: &str, key: &str) -> EventDraft {
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

#[allow(dead_code)]
pub fn usage(agent: &str, tokens: i64, ts: i64) -> EventDraft {
    EventDraft {
        event_ts_ms: ts,
        kind: EventKind::UsageRecorded,
        subject: Subject::Agent(AgentId(agent.to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: serde_json::json!({ "tokens": tokens }),
    }
}

#[allow(dead_code)]
pub async fn test_ingestor() -> Ingestor {
    let (store, _dir) = test_store().await;
    // leak dir? tests only need store; keep dir alive by leaking (acceptable in tests)
    std::mem::forget(_dir);
    Ingestor::new(store)
}

#[allow(dead_code)]
pub fn log_with_raw(agent: &str, msg: &str, raw: &str) -> EventDraft {
    let mut d = EventDraft {
        event_ts_ms: 1000,
        kind: EventKind::LogRecorded,
        subject: Subject::Agent(AgentId(agent.to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: serde_json::json!({ "message": msg, "sentence": { "key": "log", "params": { "detail": raw } } }),
    };
    // ensure raw present
    d.payload["sentence"]["params"]["detail"] = raw.into();
    d
}

#[allow(dead_code)]
pub fn webhook_draft(source: &str, delivery: &str) -> EventDraft {
    EventDraft {
        event_ts_ms: 1000,
        kind: EventKind::ConnectorEvent,
        subject: Subject::System,
        source: SourceId(source.to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(delivery.to_string()),
        payload: serde_json::json!({ "data": delivery }),
    }
}
