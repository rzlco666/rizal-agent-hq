#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_core::*;
use hq_store::*;

#[tokio::test]
async fn agent_observed_upserts_card() {
    let (store, _dir) = test_store().await;
    store
        .append(vec![agent_observed("agent:omp", "bekerja")])
        .await
        .unwrap();
    let a = store
        .agent(&AgentId::parse("agent:omp").unwrap(), None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.card.status, Status::Bekerja);
    assert_eq!(a.card.name, "OMP");
}

#[tokio::test]
async fn agent_observed_persists_payload_runtime_key() {
    let (store, dir) = test_store().await;
    let mut draft = agent_observed("agent:omp", "bekerja");
    draft.payload["runtime_key"] = serde_json::json!("presence:omp");
    store.append(vec![draft]).await.unwrap();

    let conn = rusqlite::Connection::open(dir.path().join("hq.db")).unwrap();
    let runtime_key: String = conn
        .query_row(
            "SELECT runtime_key FROM agents WHERE id = 'agent:omp'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(runtime_key, "presence:omp");
}

#[tokio::test]
async fn usage_is_aggregated_per_minute_and_day() {
    let (store, _dir) = test_store().await;
    store
        .append(vec![usage("agent:omp", 1_000, 1_700_000_000_000)])
        .await
        .unwrap();
    let r = store
        .usage(UsageQuery::agent_minutes("agent:omp"))
        .await
        .unwrap();
    assert_eq!(r.total.input_tokens, 1_000);
}

#[tokio::test]
async fn patches_are_emitted_after_commit() {
    let (store, _dir) = test_store().await;
    let mut rx = store.patches();
    store
        .append(vec![agent_observed("agent:omp", "bekerja")])
        .await
        .unwrap();
    let p = rx.recv().await.unwrap();
    assert!(
        p.upserts
            .iter()
            .any(|u| matches!(u, PatchEntity::Agent(_, _, _)))
    );
}

#[tokio::test]
async fn read_since_returns_in_seq_order() {
    let (store, _dir) = test_store().await;
    let e = store
        .append(vec![
            heartbeat("agent:muse", "a"),
            heartbeat("agent:muse", "b"),
        ])
        .await
        .unwrap();
    let got = store.read_since(Seq(0), 10).await.unwrap();
    assert_eq!(
        got.iter().map(|x| x.seq).collect::<Vec<_>>(),
        vec![e[0].seq, e[1].seq]
    );
}

#[tokio::test]
async fn read_since_does_not_decode_rows_after_limit() {
    let (store, dir) = test_store().await;
    let first = store
        .append(vec![heartbeat("agent:muse", "first")])
        .await
        .unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("hq.db")).unwrap();
    conn.execute(
        "INSERT INTO events (seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,payload) \
         VALUES (2,'bad',1000,1000,'agent_seen','agent:muse','test',1,X'00')",
        [],
    )
    .unwrap();

    let got = store.read_since(Seq(0), 1).await.unwrap();

    assert_eq!(got.len(), 1);
    assert_eq!(got[0].seq, first[0].seq);
}

fn log_event(agent: &str, msg: &str, _lang: &str) -> EventDraft {
    EventDraft {
        event_ts_ms: 1000,
        kind: EventKind::LogRecorded,
        subject: Subject::Agent(AgentId(agent.to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: serde_json::json!({
            "message": msg,
            "level": "info",
            "sentence": {
                "key": "agent.running_tests",
                "params": {
                    "agent": "OMP",
                    "object": "strut-stack-sql",
                    "detail": "188 tes hijau"
                }
            }
        }),
    }
}

#[tokio::test]
async fn logs_render_in_requested_locale() {
    let (store, _dir) = test_store().await;
    store
        .append(vec![log_event("agent:omp", "menjalankan tes", "id")])
        .await
        .unwrap();
    let agent = AgentId::parse("agent:omp").unwrap();
    let id = store
        .logs(&agent, Locale::Id, 10, Default::default())
        .await
        .unwrap();
    let en = store
        .logs(&agent, Locale::En, 10, Default::default())
        .await
        .unwrap();
    assert_eq!(id.len(), 1);
    assert_eq!(en.len(), 1);
    assert!(id[0].text.contains("menjalankan tes"));
    assert!(
        en[0].text.contains("running tests") || en[0].text.contains("tests"),
        "expected English phrase, got {}",
        en[0].text
    );
}

#[tokio::test]
async fn stored_log_keeps_template_and_params_not_final_text() {
    let (store, dir) = test_store().await;
    store
        .append(vec![log_event("agent:omp", "menjalankan tes", "id")])
        .await
        .unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("hq.db")).unwrap();
    let json: String = conn
        .query_row("SELECT sentence_json FROM logs", [], |r| r.get(0))
        .unwrap();
    let sentence: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(sentence["key"], "agent.running_tests");
    assert_eq!(sentence["params"]["object"], "strut-stack-sql");
    assert!(!json.contains("running tests"), "finalized text was stored");
}

fn session_draft(kind: EventKind, ts: i64, payload: serde_json::Value) -> EventDraft {
    EventDraft {
        event_ts_ms: ts,
        kind,
        subject: Subject::Session(SessionId("agent:omp/session/s1".to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload,
    }
}

#[tokio::test]
async fn tasks_are_derived_from_session_events_without_task_events() {
    let (store, dir) = test_store().await;
    store
        .append(vec![
            session_draft(
                EventKind::SessionStarted,
                1000,
                serde_json::json!({"started_at_ms": 1000}),
            ),
            session_draft(
                EventKind::LogRecorded,
                1100,
                serde_json::json!({
                    "message": "membaca",
                    "verb": "reading",
                    "sentence": {"key": "agent.reading", "params": {"agent": "OMP", "object": "00-design.md"}}
                }),
            ),
            session_draft(
                EventKind::SessionEnded,
                1200,
                serde_json::json!({"ended_at_ms": 1200, "outcome": "ok"}),
            ),
        ])
        .await
        .unwrap();

    let conn = rusqlite::Connection::open(dir.path().join("hq.db")).unwrap();
    let id: String = conn
        .query_row("SELECT id FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(id, "task:agent:omp:s1:1000");
    let phase: String = conn
        .query_row("SELECT phase FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(phase, "selesai");
    let steps: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_steps", [], |r| r.get(0))
        .unwrap();
    assert_eq!(steps, 1);
    let task_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE kind LIKE 'task%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(task_events, 0);
}
