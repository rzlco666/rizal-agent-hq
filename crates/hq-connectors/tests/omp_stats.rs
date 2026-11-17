use hq_connectors::probes::ProcessInfo;
use hq_connectors::runtime::omp::{
    omp_status, omp_usage_events, OmpAdapter, OmpActivity, OmpMessageMeta, OmpStats,
};
use hq_connectors::runtime::{CollectCtx, RuntimeAdapter};
use hq_core::EventKind;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn stats() -> OmpStats {
    serde_json::from_str(include_str!("fixtures/omp-stats.json")).unwrap()
}

fn ctx(now_ms: i64) -> CollectCtx {
    CollectCtx {
        now_ms,
        processes: Vec::new(),
    }
}

#[test]
fn maps_overall_and_models() {
    let s = stats();
    let d = omp_usage_events(&s, 1_000_000);
    let total = d
        .iter()
        .find(|e| e.subject.key() == "usage:global")
        .unwrap();
    assert_eq!(total.payload["input_tokens"], s.overall.total_input_tokens);
    assert!(d.iter().any(|e| e.subject.key() == "agent:omp"));
    assert!(d
        .iter()
        .any(|e| e.kind == EventKind::UsageRecorded && e.payload["provider"] == "anthropic"));
    for event in &d {
        event.validate().unwrap();
    }
}

#[test]
fn absent_quota_is_never_invented() {
    let d = omp_usage_events(&stats(), 1_000_000);
    assert!(!d.iter().any(|e| e.kind == EventKind::QuotaObserved));
}

#[tokio::test]
async fn stats_command_result_is_cached_for_five_minutes() {
    let dir = std::env::temp_dir().join(format!("hq-omp-stats-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let counter = dir.join("calls");
    fs::write(&counter, "0").unwrap();
    let script = dir.join("fake-omp");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/omp-stats.json")
        .display()
        .to_string();
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"stats\" ] && [ \"$2\" = \"--json\" ]; then\n  n=$(cat {})\n  echo $((n + 1)) > {}\n  cat {fixture}\nelse\n  exit 2\nfi\n",
            counter.display(),
            counter.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let adapter = OmpAdapter::with_command(script.to_string_lossy().into_owned());
    let first = adapter.observe(&ctx(1_000_000)).await.unwrap();
    let second = adapter.observe(&ctx(1_060_000)).await.unwrap();
    assert!(!first.is_empty());
    assert_eq!(first.len(), second.len());
    assert_eq!(fs::read_to_string(&counter).unwrap().trim(), "1");
    for event in first.iter().chain(second.iter()) {
        event.validate().unwrap();
    }
    fs::remove_dir_all(&dir).unwrap();
}

fn proc(name: &str) -> Option<ProcessInfo> {
    Some(ProcessInfo {
        pid: 7,
        ppid: 1,
        pcpu: 2.5,
        pmem: 0.4,
        etime: "00:10".to_owned(),
        comm: name.to_owned(),
        args: format!("/usr/bin/{name} stats"),
    })
}

fn recent_msg(stop_reason: &str, at_ms: i64) -> Option<OmpMessageMeta> {
    Some(OmpMessageMeta {
        timestamp_ms: at_ms,
        stop_reason: Some(stop_reason.to_owned()),
        error_message: None,
        folder: Some("/mnt/d/laragon/integra/demo-epm".to_owned()),
    })
}

fn err_msg(at_ms: i64) -> Option<OmpMessageMeta> {
    Some(OmpMessageMeta {
        timestamp_ms: at_ms,
        stop_reason: Some("stop".to_owned()),
        error_message: Some("request failed".to_owned()),
        folder: None,
    })
}

#[test]
fn recent_tool_use_is_working() {
    let a = omp_status(proc("omp"), recent_msg("toolUse", 1_000_000), 1_000_000);
    assert_eq!(a.status, hq_core::Status::Bekerja);
}

#[test]
fn stopped_recent_session_is_santai() {
    let a = omp_status(
        proc("omp"),
        recent_msg("stop", 1_000_000 - 20 * 60_000),
        1_000_000,
    );
    assert_eq!(a.status, hq_core::Status::Santai);
}

#[test]
fn error_message_is_error() {
    let a = omp_status(proc("omp"), err_msg(1_000_000), 1_000_000);
    assert_eq!(a.status, hq_core::Status::Error);
}

#[test]
fn absent_process_is_offline() {
    let a = omp_status(None, recent_msg("toolUse", 1_000_000), 1_000_000);
    assert_eq!(a.status, hq_core::Status::Offline);
}

#[test]
fn recent_non_tool_message_is_thinking() {
    let a = omp_status(proc("omp"), recent_msg("stop", 1_000_000), 1_000_000);
    assert_eq!(a.status, hq_core::Status::Berpikir);
}

#[test]
fn activity_and_evidence_come_from_structured_metadata() {
    let a: OmpActivity = omp_status(
        proc("omp"),
        recent_msg("toolUse", 1_000_000),
        1_000_000,
    );
    assert_eq!(a.activity.key, "agent.working_in_folder");
    assert_eq!(
        a.activity.params.get("folder").map(String::as_str),
        Some("/mnt/d/laragon/integra/demo-epm")
    );
    assert_eq!(a.evidence, hq_core::EvidenceLevel::L3);
}
