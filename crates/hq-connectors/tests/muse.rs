use hq_connectors::runtime::muse::{MuseAdapter, parse_heartbeat};
use hq_connectors::runtime::{CollectCtx, RuntimeAdapter};
use hq_core::{EventKind, EvidenceLevel};
use serde_json::json;
use std::fs;

const FIXTURE: &str = include_str!("fixtures/muse-heartbeat.json");

fn ctx_at(now_ms: i64) -> CollectCtx {
    CollectCtx {
        now_ms,
        processes: Vec::new(),
    }
}

fn fixture_heartbeats() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("heartbeats")).unwrap();
    fs::write(dir.path().join("heartbeats/muse.json"), FIXTURE).unwrap();
    dir
}

#[test]
fn parses_v2_heartbeat() {
    let h = parse_heartbeat(FIXTURE).unwrap();
    assert_eq!(h.agent, "muse");
    assert_eq!(h.at, 940_000);
    assert_eq!(h.status, "bekerja");
    assert_eq!(h.role, "assistant");
}

#[tokio::test]
async fn fresh_heartbeat_is_working() {
    let out = MuseAdapter::new(fixture_heartbeats().path())
        .observe(&ctx_at(1_000_000))
        .await
        .unwrap();
    assert!(out.iter().any(|d| d.payload["status"] == "bekerja"));
    let event = out
        .iter()
        .find(|d| d.subject.key() == "agent:muse")
        .unwrap();
    assert_eq!(event.kind, EventKind::AgentObserved);
    assert_eq!(
        event.payload["activity"]["params"]["text"],
        "Meninjau hasil build dan menyiapkan catatan rilis"
    );
    assert_eq!(event.payload["evidence"], json!(EvidenceLevel::L2));
    for event in &out {
        event.validate().unwrap();
    }
}

#[tokio::test]
async fn stale_heartbeat_is_offline() {
    let out = MuseAdapter::new(fixture_heartbeats().path())
        .observe(&ctx_at(1_000_000 + 11 * 60_000))
        .await
        .unwrap();
    assert!(out.iter().any(|d| d.payload["status"] == "offline"));
}

#[tokio::test]
async fn heartbeat_task_is_redacted_before_emission() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("heartbeats")).unwrap();
    let heartbeat = json!({
        "agent": "muse",
        "at": 1_000_000,
        "role": "assistant",
        "status": "bekerja",
        "task": "Token ghp_abcdefghijklmnop untuk deploy"
    });
    fs::write(
        dir.path().join("heartbeats/muse.json"),
        serde_json::to_string(&heartbeat).unwrap(),
    )
    .unwrap();
    let out = MuseAdapter::new(dir.path())
        .observe(&ctx_at(1_000_000))
        .await
        .unwrap();
    let all = serde_json::to_string(&out.iter().map(|d| &d.payload).collect::<Vec<_>>()).unwrap();
    assert!(!all.contains("ghp_abcdefghijklmnop"));
    assert!(all.contains("[REDACTED:github]"));
}

#[tokio::test]
async fn missing_heartbeats_are_empty() {
    let dir = tempfile::tempdir().unwrap();
    let out = MuseAdapter::new(dir.path())
        .observe(&ctx_at(1_000_000))
        .await
        .unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn heartbeat_role_is_redacted_too() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("heartbeats")).unwrap();
    let heartbeat = json!({
        "agent": "muse",
        "at": 1_000_000,
        "role": "ghp_abcdefghijklmnop",
        "status": "bekerja",
        "task": "meninjau rilis"
    });
    fs::write(
        dir.path().join("heartbeats/muse.json"),
        serde_json::to_string(&heartbeat).unwrap(),
    )
    .unwrap();
    let out = MuseAdapter::new(dir.path())
        .observe(&ctx_at(1_000_000))
        .await
        .unwrap();
    let event = out
        .iter()
        .find(|d| d.subject.key() == "agent:muse")
        .unwrap();
    assert_eq!(event.payload["role"], "[REDACTED:github]");
    assert!(
        !serde_json::to_string(&event.payload)
            .unwrap()
            .contains("ghp_abcdefghijklmnop")
    );
}
