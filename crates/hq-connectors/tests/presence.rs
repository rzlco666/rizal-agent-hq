use hq_connectors::probes::ProcessInfo;
use hq_connectors::runtime::{CollectCtx, ProcessPresenceAdapter, RuntimeAdapter};
use hq_core::EventKind;

fn proc(comm: &str, args: &str) -> ProcessInfo {
    ProcessInfo {
        pid: 42,
        ppid: 1,
        pcpu: 0.0,
        pmem: 0.0,
        etime: "00:01".into(),
        comm: comm.into(),
        args: args.into(),
    }
}

fn ctx_with_procs(processes: Vec<ProcessInfo>, now_ms: i64) -> CollectCtx {
    CollectCtx { now_ms, processes }
}

#[tokio::test]
async fn omp_process_creates_l1_agent() {
    let ctx = ctx_with_procs(vec![proc("omp", "omp --model x")], 1_000);
    let events = ProcessPresenceAdapter::default()
        .observe(&ctx)
        .await
        .unwrap();
    let event = events
        .iter()
        .find(|event| event.subject.key() == "agent:omp")
        .unwrap();
    assert_eq!(event.kind, EventKind::AgentObserved);
    assert_eq!(event.payload["status"], "bekerja");
    assert_eq!(event.payload["evidence"], "L1");
    assert_eq!(event.payload["runtime_key"], "presence:omp");
    assert_eq!(event.payload["activity"]["key"], "agent.process_running");
    assert!(event.validate().is_ok());
}

#[tokio::test]
async fn claude_process_uses_canonical_agent_subject() {
    let ctx = ctx_with_procs(vec![proc("claude", "claude --model x")], 1_000);
    let events = ProcessPresenceAdapter::default()
        .observe(&ctx)
        .await
        .unwrap();
    let event = events
        .iter()
        .find(|event| event.subject.key() == "agent:claude-code")
        .unwrap();
    assert_eq!(event.payload["runtime_key"], "presence:claude");
    assert_eq!(event.payload["activity"]["params"]["family"], "claude");
    assert!(event.validate().is_ok());
}

#[tokio::test]
async fn gemini_process_uses_canonical_agent_subject() {
    let ctx = ctx_with_procs(vec![proc("gemini", "gemini cli")], 1_000);
    let events = ProcessPresenceAdapter::default()
        .observe(&ctx)
        .await
        .unwrap();
    let event = events
        .iter()
        .find(|event| event.subject.key() == "agent:gemini-cli")
        .unwrap();
    assert_eq!(event.payload["runtime_key"], "presence:gemini");
    assert_eq!(event.payload["activity"]["params"]["family"], "gemini");
    assert!(event.validate().is_ok());
}

#[tokio::test]
async fn l1_agent_never_claims_a_task() {
    let ctx = ctx_with_procs(vec![proc("cursor", "cursor")], 1_000);
    let events = ProcessPresenceAdapter::default()
        .observe(&ctx)
        .await
        .unwrap();
    let payloads = events
        .iter()
        .map(|event| &event.payload)
        .collect::<Vec<_>>();
    assert!(!serde_json::to_string(&payloads).unwrap().contains("task."));
}

#[tokio::test]
async fn collector_survives_disappearing_source() {
    let adapter = ProcessPresenceAdapter::default();
    adapter
        .observe(&ctx_with_procs(vec![proc("omp", "omp")], 1_000))
        .await
        .unwrap();
    assert!(
        adapter
            .observe(&ctx_with_procs(vec![], 2_000))
            .await
            .unwrap()
            .is_empty()
    );
    let events = adapter
        .observe(&ctx_with_procs(vec![], 3_000))
        .await
        .unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.payload["status"] == "offline")
    );
}

#[tokio::test]
async fn active_process_emits_one_observation_until_state_changes() {
    let adapter = ProcessPresenceAdapter::default();
    let active = || ctx_with_procs(vec![proc("omp", "omp")], 1_000);
    assert_eq!(adapter.observe(&active()).await.unwrap().len(), 1);
    assert!(adapter.observe(&active()).await.unwrap().is_empty());
}

#[tokio::test]
async fn system_health_emits_no_more_than_every_thirty_seconds() {
    use hq_connectors::probes::{Probes, SystemMetrics};
    use hq_connectors::runtime::{Clock, SystemHealthEmitter};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    #[derive(Clone)]
    struct TestClock(Arc<AtomicI64>);
    impl Clock for TestClock {
        fn now_ms(&self) -> i64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    let now = Arc::new(AtomicI64::new(1_000_000));
    let emitter = SystemHealthEmitter::new(TestClock(now.clone()));
    let probes = Probes {
        system: Some(SystemMetrics {
            cpu_percent: 10.0,
            mem_percent: 40.0,
            disk_percent: 85.0,
            uptime_secs: 85,
        }),
        ..Probes::default()
    };
    assert!(emitter.observe(&probes).is_some());
    now.store(1_010_000, Ordering::SeqCst);
    assert!(emitter.observe(&probes).is_none());
    now.store(1_031_000, Ordering::SeqCst);
    let event = emitter.observe(&probes).unwrap();
    assert_eq!(event.payload["disk_percent"], 85.0);
    assert!(event.payload.get("docker").is_some());
    assert!(event.payload.get("git").is_some());
    assert!(event.validate().is_ok());
}
