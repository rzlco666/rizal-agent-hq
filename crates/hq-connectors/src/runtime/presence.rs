use crate::probes::{Probes, ProcessInfo};
use async_trait::async_trait;
use hq_core::{
    AgentId, AgentKind, EventDraft, EventKind, EvidenceLevel, Sentence, SourceId, Status, Subject,
};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

#[derive(Clone, Debug, Default)]
pub struct CollectCtx {
    pub now_ms: i64,
    pub processes: Vec<ProcessInfo>,
}

#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    async fn observe(&self, ctx: &CollectCtx) -> Result<Vec<EventDraft>, String>;
}

#[derive(Default)]
pub struct ProcessPresenceAdapter {
    absent_cycles: Mutex<HashMap<String, u8>>,
}

#[async_trait]
impl RuntimeAdapter for ProcessPresenceAdapter {
    async fn observe(&self, ctx: &CollectCtx) -> Result<Vec<EventDraft>, String> {
        let mut by_family: BTreeMap<String, Vec<&ProcessInfo>> = BTreeMap::new();
        for process in &ctx.processes {
            if let Some(family) = classify(process) {
                by_family
                    .entry(family.to_owned())
                    .or_default()
                    .push(process);
            }
        }
        let mut absent = self
            .absent_cycles
            .lock()
            .map_err(|_| "presence state unavailable".to_owned())?;
        let mut events = Vec::new();
        for (family, processes) in &by_family {
            let previous = absent.insert(family.clone(), 0);
            if previous.is_none_or(|cycles| cycles >= 2) {
                events.push(agent_event(ctx.now_ms, family, Status::Bekerja, processes));
            }
        }
        for (family, cycles) in absent.iter_mut() {
            if by_family.contains_key(family) || *cycles >= 2 {
                continue;
            }
            *cycles += 1;
            if *cycles == 2 {
                events.push(agent_event(ctx.now_ms, family, Status::Offline, &[]));
            }
        }
        Ok(events)
    }
}

fn agent_event(
    now_ms: i64,
    family: &str,
    status: Status,
    processes: &[&ProcessInfo],
) -> EventDraft {
    let mut params = BTreeMap::new();
    params.insert("family".to_owned(), family.to_owned());
    let activity = Sentence {
        key: "agent.process_running".to_owned(),
        params,
    };
    let payload = json!({
        "observed_at_ms": now_ms,
        "runtime_key": format!("presence:{family}"),
        "status": status,
        "kind": if family == "mcp" { AgentKind::Mcp } else { AgentKind::Utama },
        "activity": activity,
        "processes": processes.len() as u32,
        "cpu_percent": processes.iter().map(|process| process.pcpu).sum::<f32>(),
        "evidence": EvidenceLevel::L1,
    });
    EventDraft {
        event_ts_ms: now_ms,
        kind: EventKind::AgentObserved,
        subject: Subject::Agent(AgentId(format!("agent:{family}"))),
        source: SourceId("local".to_owned()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(format!("presence:{family}:{now_ms}")),
        payload,
    }
}

pub fn classify(process: &ProcessInfo) -> Option<&'static str> {
    let searchable = format!("{} {}", process.comm, process.args).to_ascii_lowercase();
    let families = [
        "omp", "cursor", "claude", "codex", "devin", "gemini", "opencode", "copilot",
    ];
    families
        .into_iter()
        .find(|family| searchable.contains(family))
        .or_else(|| {
            (searchable.contains("mcp-server")
                || searchable.contains("mcp server")
                || searchable.contains("mcp_"))
            .then_some("mcp")
        })
}

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}

pub struct SystemHealthEmitter<C: Clock> {
    clock: C,
    last_emitted_ms: Mutex<Option<i64>>,
}

impl<C: Clock> SystemHealthEmitter<C> {
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            last_emitted_ms: Mutex::new(None),
        }
    }

    pub fn observe(&self, probes: &Probes) -> Option<EventDraft> {
        let system = probes.system.as_ref()?;
        let now_ms = self.clock.now_ms();
        let mut last = self.last_emitted_ms.lock().ok()?;
        if last.is_some_and(|previous| now_ms.saturating_sub(previous) < 30_000) {
            return None;
        }
        *last = Some(now_ms);
        Some(EventDraft {
            event_ts_ms: now_ms,
            kind: EventKind::SystemHealthObserved,
            subject: Subject::System,
            source: SourceId("local".to_owned()),
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(format!("system-health:{now_ms}")),
            payload: json!({
                "cpu_percent": system.cpu_percent,
                "mem_percent": system.mem_percent,
                "disk_percent": system.disk_percent,
                "uptime_secs": system.uptime_secs,
                "docker": probes.docker,
                "git": probes.git,
            }),
        })
    }
}
