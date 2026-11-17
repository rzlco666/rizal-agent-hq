use hq_connectors::probes::Probes;
use hq_connectors::runtime::{
    Clock, CollectCtx, ProcessPresenceAdapter, RuntimeAdapter, SystemHealthEmitter,
};
use hq_core::{EventDraft, EventKind};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct WallClock;
impl Clock for WallClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default()
    }
}

pub async fn run(args: &[String]) -> i32 {
    if args != ["collect", "--once", "--json"] {
        eprintln!("usage: rizal-agent-hq collect --once --json");
        return 2;
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    // No repository configuration is registered yet; an empty list deliberately skips Git probes.
    let probes = Probes::collect(&[], &home);
    let now_ms = WallClock.now_ms();
    let adapter = ProcessPresenceAdapter::default();
    let ctx = CollectCtx {
        now_ms,
        processes: probes.processes.clone(),
    };
    let mut events = adapter.observe(&ctx).await.unwrap_or_default();
    if let Some(health) = SystemHealthEmitter::new(WallClock).observe(&probes) {
        events.push(health);
    }

    let mut output = serde_json::to_value(&probes).unwrap_or_default();
    if let Some(object) = output.as_object_mut() {
        object.insert(
            "events".to_owned(),
            serde_json::Value::Array(events.iter().map(event_json).collect()),
        );
    }
    println!("{output}");
    if probes.all_failed() { 1 } else { 0 }
}

fn event_json(event: &EventDraft) -> serde_json::Value {
    serde_json::json!({
        "event_ts_ms": event.event_ts_ms,
        "kind": if event.kind == EventKind::SystemHealthObserved { "system_health_observed" } else { "agent_observed" },
        "subject": event.subject.key(),
        "source": event.source.0,
        "payload": event.payload,
    })
}
