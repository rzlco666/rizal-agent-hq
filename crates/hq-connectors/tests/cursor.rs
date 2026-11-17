use hq_connectors::probes::ProcessInfo;
use hq_connectors::runtime::cursor::{parse_cursor_terminal, CursorAdapter};
use hq_connectors::runtime::{CollectCtx, RuntimeAdapter};
use hq_core::{EventKind, EvidenceLevel};
use std::fs;
use std::time::{Duration, SystemTime};

const HEADER: &str = include_str!("fixtures/cursor-terminal.txt");

fn ctx() -> CollectCtx {
    CollectCtx {
        now_ms: 1_000_000,
        processes: Vec::<ProcessInfo>::new(),
    }
}

fn terminal(status: &str, title: &str) -> String {
    HEADER
        .replace("status: succeeded", &format!("status: {status}"))
        .replace(
            "title: \"Check pwrstg route and login.css references\"",
            &format!("title: \"{title}\""),
        )
}

fn fixture_cursor_dir(workspaces: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for index in 0..workspaces {
        let terminals = dir.path().join(format!("projects/ws-{index}/terminals"));
        fs::create_dir_all(&terminals).unwrap();
        let status = match index % 3 {
            0 => "running",
            1 => "succeeded",
            _ => "failed",
        };
        let path = terminals.join("terminal.txt");
        fs::write(&path, terminal(status, &format!("Workspace {index} review"))).unwrap();
        let mtime = SystemTime::now() - Duration::from_secs(index as u64 * 60);
        fs::File::open(&path).unwrap().set_modified(mtime).unwrap();
    }
    dir
}

#[test]
fn parses_cursor_terminal_header() {
    let t = parse_cursor_terminal(HEADER).unwrap();
    assert_eq!(t.title, "Check pwrstg route and login.css references");
    assert_eq!(t.status, "succeeded");
    assert_eq!(t.pid, Some(229943));
    assert_eq!(t.cwd.as_deref(), Some("/mnt/d/laragon/integra/demo-epm/img"));
    assert_eq!(t.running_for_ms, Some(65_042));
}

#[tokio::test]
async fn active_terminal_sets_working_and_failed_sets_error() {
    let dir = fixture_cursor_dir(3);
    let out = CursorAdapter::new(dir.path()).observe(&ctx()).await.unwrap();
    assert!(out
        .iter()
        .any(|d| d.payload["status"] == "bekerja" && d.subject.key().contains("ws-0")));
    assert!(out
        .iter()
        .any(|d| d.payload["status"] == "santai" && d.subject.key().contains("ws-1")));
    assert!(out
        .iter()
        .any(|d| d.payload["status"] == "error" && d.subject.key().contains("ws-2")));
    for event in &out {
        event.validate().unwrap();
    }
}

#[tokio::test]
async fn terminal_title_becomes_activity_and_evidence_is_l2() {
    let dir = fixture_cursor_dir(1);
    let out = CursorAdapter::new(dir.path()).observe(&ctx()).await.unwrap();
    let event = out
        .iter()
        .find(|d| d.subject.key() == "agent:cursor/workspace/ws-0")
        .unwrap();
    assert_eq!(event.kind, EventKind::AgentObserved);
    assert_eq!(event.payload["activity"]["key"], "agent.cursor_activity");
    assert_eq!(
        event.payload["activity"]["params"]["title"],
        "Workspace 0 review"
    );
    assert_eq!(event.payload["evidence"], serde_json::json!(EvidenceLevel::L2));
}

#[tokio::test]
async fn no_token_claims_from_cursor() {
    let dir = fixture_cursor_dir(2);
    let out = CursorAdapter::new(dir.path()).observe(&ctx()).await.unwrap();
    assert!(!out.iter().any(|d| matches!(
        d.kind,
        EventKind::UsageRecorded | EventKind::QuotaObserved | EventKind::CostEstimated
    )));
}

#[tokio::test]
async fn workspaces_are_capped_at_five_plus_aggregate() {
    let dir = fixture_cursor_dir(8);
    let out = CursorAdapter::new(dir.path()).observe(&ctx()).await.unwrap();
    let workspaces: Vec<_> = out
        .iter()
        .filter(|d| d.subject.key().starts_with("agent:cursor/workspace/"))
        .collect();
    assert_eq!(workspaces.len(), 5);
    assert!(out.iter().any(|d| d.subject.key() == "agent:cursor"));
}

#[tokio::test]
async fn missing_cursor_root_is_empty_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = CursorAdapter::new(dir.path().join("missing"))
        .observe(&ctx())
        .await
        .unwrap();
    assert!(out.is_empty());
}
