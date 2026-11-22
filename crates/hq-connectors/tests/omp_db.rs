use hq_connectors::probes::ProcessInfo;
use hq_connectors::runtime::omp_db::OmpDbAdapter;
use hq_connectors::runtime::{CollectCtx, RuntimeAdapter};
use hq_core::EventKind;
use rusqlite::Connection;
use serde_json::json;
use std::time::Instant;

const NOW: i64 = 1_000_000;

fn ctx() -> CollectCtx {
    CollectCtx {
        now_ms: NOW,
        processes: vec![ProcessInfo {
            pid: 42,
            ppid: 1,
            pcpu: 1.0,
            pmem: 0.1,
            etime: "01:00".to_owned(),
            comm: "omp".to_owned(),
            args: "bun /home/rizal/.bun/bin/omp".to_owned(),
        }],
    }
}

fn fixture_omp_db() -> tempfile::TempDir {
    fixture_omp_db_with_worker_types(&["researcher"])
}

fn fixture_omp_db_with_worker_types(worker_types: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let conn = Connection::open(dir.path().join("stats.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE session_rollup (
            session_file TEXT, requests INTEGER, started_at INTEGER, ended_at INTEGER,
            total_tokens INTEGER, cost_total REAL, unpriced INTEGER, models TEXT, tool_calls INTEGER
         );
         CREATE TABLE messages (
            id INTEGER PRIMARY KEY, session_file TEXT, model TEXT, provider TEXT,
            timestamp INTEGER, stop_reason TEXT, error_message TEXT, agent_type TEXT,
            tokens INTEGER, cost REAL, content TEXT
         );
         CREATE TABLE tool_calls (
            id INTEGER PRIMARY KEY, session_file TEXT, name TEXT, timestamp INTEGER
         );",
    )
    .unwrap();
    for (file, requests, tokens, cost) in [
        ("session-a.jsonl", 3, 12_000, 0.12),
        ("session-b.jsonl", 2, 8_000, 0.08),
    ] {
        conn.execute(
            "INSERT INTO session_rollup VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 'anthropic:claude', 4)",
            rusqlite::params![file, requests, NOW - 600_000, NOW - 300_000, tokens, cost],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO messages
         (session_file, model, provider, timestamp, stop_reason, error_message, agent_type, tokens, cost, content)
         VALUES ('session-a.jsonl', 'claude-haiku-4-5', 'anthropic', ?1, 'toolUse', NULL, 'main', 4000, 0.04, 'main content')",
        rusqlite::params![NOW - 30_000],
    )
    .unwrap();
    for (index, worker_type) in worker_types.iter().enumerate() {
        conn.execute(
            "INSERT INTO messages
             (session_file, model, provider, timestamp, stop_reason, error_message, agent_type, tokens, cost, content)
             VALUES ('session-a.jsonl', 'claude-haiku-4-5', 'anthropic', ?1, 'toolUse', NULL, ?2, ?3, 0.02, 'isi percakapan rahasia')",
            rusqlite::params![NOW - 60_000 + index as i64, worker_type, 2000 + index as i64],
        )
        .unwrap();
    }
    conn.close().unwrap();
    dir
}

#[tokio::test]
async fn session_rollup_becomes_session_and_usage() {
    let db = fixture_omp_db();
    let out = OmpDbAdapter::new(db.path().join("stats.db"))
        .observe(&ctx())
        .await
        .unwrap();
    assert!(
        out.iter()
            .any(|d| d.kind == EventKind::SessionStarted && d.payload["started_at_ms"].is_number())
    );
    assert!(
        out.iter()
            .any(|d| d.subject.key().starts_with("agent:omp/session/"))
    );
    assert!(out.iter().any(|d| d.kind == EventKind::SessionEnded));
    assert!(out.iter().any(|d| d.kind == EventKind::UsageRecorded));
    for event in &out {
        event.validate().unwrap();
    }
}

#[tokio::test]
async fn subagent_messages_create_worker_agents() {
    let db = fixture_omp_db();
    let out = OmpDbAdapter::new(db.path().join("stats.db"))
        .observe(&ctx())
        .await
        .unwrap();
    assert!(
        out.iter()
            .any(|d| d.subject.key().starts_with("agent:omp/worker/"))
    );
    assert!(out.iter().any(|d| d.payload["worker_type"] == "researcher"));
}

#[tokio::test]
async fn no_message_content_leaks() {
    let db = fixture_omp_db();
    let out = OmpDbAdapter::new(db.path().join("stats.db"))
        .observe(&ctx())
        .await
        .unwrap();
    let all = serde_json::to_string(
        &out.iter()
            .map(|d| json!({ "subject": d.subject.key(), "payload": d.payload }))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(!all.contains("isi percakapan rahasia"));
    assert!(!all.contains("main content"));
}

#[tokio::test]
async fn workers_are_capped_at_eight_plus_aggregate() {
    let types = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];
    let db = fixture_omp_db_with_worker_types(&types);
    let out = OmpDbAdapter::new(db.path().join("stats.db"))
        .observe(&ctx())
        .await
        .unwrap();
    let workers: Vec<_> = out
        .iter()
        .filter(|d| d.subject.key().starts_with("agent:omp/worker/"))
        .collect();
    assert_eq!(workers.len(), 9);
    assert!(
        workers
            .iter()
            .any(|d| d.subject.key() == "agent:omp/worker/agg")
    );
}

#[tokio::test]
async fn aggregate_worker_uses_canonical_runtime_key() {
    let types = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    let db = fixture_omp_db_with_worker_types(&types);
    let out = OmpDbAdapter::new(db.path().join("stats.db"))
        .observe(&ctx())
        .await
        .unwrap();
    let agg = out
        .iter()
        .find(|d| d.subject.key() == "agent:omp/worker/agg")
        .unwrap();
    assert_eq!(agg.payload["runtime_key"], "omp:worker:agg");
    assert_eq!(agg.payload["worker_type"], "omp-workers-agg");
}

#[tokio::test]
async fn missing_db_does_not_hang_and_emits_nothing() {
    let db = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let out = OmpDbAdapter::new(db.path().join("missing.db"))
        .observe(&ctx())
        .await
        .unwrap();
    assert!(out.is_empty());
    assert!(started.elapsed().as_secs() < 2);
}

#[tokio::test]
async fn observation_does_not_modify_the_database() {
    let db = fixture_omp_db();
    let path = db.path().join("stats.db");
    let before = std::fs::read(&path).unwrap();
    OmpDbAdapter::new(&path).observe(&ctx()).await.unwrap();
    assert_eq!(before, std::fs::read(&path).unwrap());
}
