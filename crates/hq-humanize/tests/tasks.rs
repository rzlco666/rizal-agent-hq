use hq_core::*;
use hq_humanize::{Fact, TaskDeriver, TaskPhase, Verb, humanize};
use serde_json::json;
use ulid::Ulid;

fn event(
    kind: EventKind,
    subject: Subject,
    ts: i64,
    seq: i64,
    payload: serde_json::Value,
) -> Event {
    Event {
        seq: Seq(seq),
        id: EventId(Ulid::default()),
        ingest_ts_ms: ts,
        draft: EventDraft {
            event_ts_ms: ts,
            kind,
            subject,
            source: SourceId("test".to_string()),
            correlation_id: None,
            causation_id: None,
            idempotency_key: None,
            payload,
        },
    }
}

fn session_subject(agent: &str, session: &str) -> Subject {
    Subject::Session(SessionId(format!("{agent}/session/{session}")))
}

fn session_started(agent: &str, session: &str, ts: i64) -> Event {
    event(
        EventKind::SessionStarted,
        session_subject(agent, session),
        ts,
        ts,
        json!({"started_at_ms": ts}),
    )
}

fn session_ended(agent: &str, session: &str, ts: i64, ok: bool) -> Event {
    event(
        EventKind::SessionEnded,
        session_subject(agent, session),
        ts,
        ts,
        json!({"ended_at_ms": ts, "outcome": if ok { "ok" } else { "failed" }}),
    )
}

fn log(agent: &str, session: &str, verb: Verb, ts: i64) -> Event {
    event(
        EventKind::LogRecorded,
        session_subject(agent, session),
        ts,
        ts,
        json!({"message": "aktivitas", "verb": verb, "sentence": humanize(&Fact::new(agent, verb))}),
    )
}

fn waiting_user(agent: &str, session: &str, ts: i64) -> Event {
    log(agent, session, Verb::WaitingUser, ts)
}

fn fixture_events() -> Vec<Event> {
    vec![
        session_started("agent:omp", "s1", 1000),
        log("agent:omp", "s1", Verb::Reading, 1100),
        log("agent:omp", "s1", Verb::EditingFile, 1200),
        log("agent:omp", "s1", Verb::RunningTests, 1300),
        session_ended("agent:omp", "s1", 1400, true),
    ]
}

#[test]
fn session_becomes_one_task_with_steps() {
    let tasks = TaskDeriver::default().derive(&fixture_events());
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "task:agent:omp:s1:1000");
    assert_eq!(tasks[0].phase, TaskPhase::Selesai);
    assert_eq!(tasks[0].steps.len(), 3);
    assert_eq!(tasks[0].steps[0].label.key, "agent.reading");
    assert!(tasks[0].id.starts_with("task:agent:omp:s1:"));
}

#[test]
fn derivation_is_stable_across_runs() {
    let events = fixture_events();
    let a = TaskDeriver::default().derive(&events);
    let b = TaskDeriver::default().derive(&events);
    assert_eq!(a, b);
}

#[test]
fn waiting_event_sets_phase() {
    let events = vec![
        session_started("agent:muse", "s2", 1000),
        waiting_user("agent:muse", "s2", 1100),
    ];
    let tasks = TaskDeriver::default().derive(&events);
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].phase, TaskPhase::MenungguRizal);
}

#[test]
fn steps_are_unique_ordered_and_capped_at_six() {
    let mut events = vec![session_started("agent:omp", "s3", 1000)];
    for (i, verb) in [
        Verb::Reading,
        Verb::Reading,
        Verb::EditingFile,
        Verb::Searching,
        Verb::Building,
        Verb::RunningTests,
        Verb::Committing,
        Verb::Thinking,
    ]
    .iter()
    .enumerate()
    {
        events.push(log("agent:omp", "s3", *verb, 1100 + i as i64 * 100));
    }
    events.push(session_ended("agent:omp", "s3", 2000, false));
    let tasks = TaskDeriver::default().derive(&events);
    assert_eq!(tasks[0].phase, TaskPhase::Gagal);
    assert_eq!(tasks[0].steps.len(), 6);
    let keys: Vec<_> = tasks[0]
        .steps
        .iter()
        .map(|s| s.label.key.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "agent.reading",
            "agent.editing_file",
            "agent.searching",
            "agent.building",
            "agent.running_tests",
            "agent.committing"
        ]
    );
}

#[test]
fn unrelated_events_do_not_create_tasks() {
    let events = vec![event(
        EventKind::SystemHealthObserved,
        Subject::System,
        1000,
        1,
        json!({}),
    )];
    assert!(TaskDeriver::default().derive(&events).is_empty());
}
