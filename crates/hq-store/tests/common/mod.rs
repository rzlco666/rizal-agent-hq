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
    format!("{}", store.count_events().await.unwrap())
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
        payload: serde_json::json!({ "observed_at_ms": 1000, "status": status }),
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
