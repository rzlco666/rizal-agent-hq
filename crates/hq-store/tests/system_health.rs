#![allow(unused_imports, dead_code)]
mod common;
use common::test_store;
use hq_core::*;
use hq_store::*;
use serde_json::json;

fn health(at_ms: i64, disk_percent: f64) -> EventDraft {
    EventDraft {
        event_ts_ms: at_ms,
        kind: EventKind::SystemHealthObserved,
        subject: Subject::System,
        source: SourceId("local".into()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: json!({
            "cpu_percent": 10.0,
            "mem_percent": 40.0,
            "disk_percent": disk_percent,
            "uptime_secs": 85,
            "docker": [],
            "git": []
        }),
    }
}

#[tokio::test]
async fn projects_latest_health_and_keeps_only_24_hour_history() {
    let (store, _dir) = test_store().await;
    store
        .append(vec![health(1_000, 20.0), health(86_401_001, 30.0)])
        .await
        .unwrap();
    let report = store.system_health().await.unwrap();
    let latest = report.latest.unwrap();
    assert_eq!(latest.disk_percent, 30.0);
    assert_eq!(report.history.len(), 1);
    assert_eq!(report.history[0].ts_ms, 86_401_001);
}
