use crate::{Fact, Sentence, Verb, humanize};
use hq_core::{Event, EventKind, Ms, Subject, TaskPhase};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// One visible step of a derived task. Labels stay as template plus params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStepProjection {
    pub label: Sentence,
    pub at_ms: Ms,
}

/// A task rebuilt from existing events; no `task.*` source event is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProjection {
    pub id: String,
    pub agent_id: String,
    pub session_id: String,
    pub title: Sentence,
    pub phase: TaskPhase,
    pub started_ms: Ms,
    pub ended_ms: Option<Ms>,
    pub steps: Vec<TaskStepProjection>,
}

/// Deterministic, side-effect-free session-to-task projection.
#[derive(Debug, Clone, Copy)]
pub struct TaskDeriver {
    pub max_steps: usize,
}

impl Default for TaskDeriver {
    fn default() -> Self {
        Self { max_steps: 6 }
    }
}
impl TaskDeriver {
    pub fn derive(&self, events: &[Event]) -> Vec<TaskProjection> {
        let mut ordered: Vec<&Event> = events.iter().collect();
        ordered.sort_by_key(|ev| (ev.draft.event_ts_ms, ev.seq.0));

        let mut groups: BTreeMap<(String, String), Vec<&Event>> = BTreeMap::new();
        for ev in ordered {
            if let Some(parts) = session_parts(&ev.draft.subject) {
                groups.entry(parts).or_default().push(ev);
            }
        }
        groups
            .into_iter()
            .map(|((agent, session), evs)| derive_task(agent, session, &evs, self.max_steps))
            .collect()
    }
}

fn session_parts(subject: &Subject) -> Option<(String, String)> {
    match subject {
        Subject::Session(session) => {
            let (agent, id) = session.0.split_once("/session/")?;
            Some((agent.to_string(), id.to_string()))
        }
        Subject::Agent(agent) => Some((agent.0.clone(), String::new())),
        _ => None,
    }
}

fn derive_task(
    agent: String,
    session: String,
    events: &[&Event],
    max_steps: usize,
) -> TaskProjection {
    let started_ms = events
        .iter()
        .find(|ev| ev.draft.kind == EventKind::SessionStarted)
        .map(|ev| ev.draft.event_ts_ms)
        .or_else(|| events.first().map(|ev| ev.draft.event_ts_ms))
        .unwrap_or(0);
    let ended_ms = events
        .iter()
        .filter(|ev| ev.draft.kind == EventKind::SessionEnded)
        .map(|ev| ev.draft.event_ts_ms)
        .max();

    let mut steps = Vec::new();
    let mut seen = BTreeSet::new();
    for ev in events {
        if let Some(label) = activity_sentence(&agent, ev)
            && seen.insert(label.key.clone())
            && steps.len() < max_steps
        {
            steps.push(TaskStepProjection {
                label,
                at_ms: ev.draft.event_ts_ms,
            });
        }
    }

    let phase = events
        .iter()
        .rev()
        .find_map(|ev| phase_from_event(&agent, ev))
        .unwrap_or(TaskPhase::Berjalan);

    let title = steps
        .first()
        .map(|step| step.label.clone())
        .unwrap_or_else(|| {
            let mut fact = Fact::new(agent.clone(), Verb::StartedTask);
            if !session.is_empty() {
                fact.object = Some(session.clone());
            }
            humanize(&fact)
        });

    TaskProjection {
        id: format!("task:{agent}:{session}:{started_ms}"),
        agent_id: agent,
        session_id: session,
        title,
        phase,
        started_ms,
        ended_ms,
        steps,
    }
}

fn activity_sentence(agent: &str, ev: &Event) -> Option<Sentence> {
    if ev.draft.kind != EventKind::LogRecorded {
        return None;
    }
    if let Ok(verb) = serde_json::from_value::<Verb>(ev.draft.payload["verb"].clone()) {
        let mut fact = Fact::new(agent, verb);
        fact.object = string_field(&ev.draft.payload, "object");
        fact.detail = string_field(&ev.draft.payload, "detail");
        fact.outcome = string_field(&ev.draft.payload, "outcome");
        return Some(humanize(&fact));
    }
    serde_json::from_value::<Sentence>(ev.draft.payload["sentence"].clone()).ok()
}

fn phase_from_event(agent: &str, ev: &Event) -> Option<TaskPhase> {
    match ev.draft.kind {
        EventKind::SessionStarted | EventKind::SessionUpdated => Some(TaskPhase::Berjalan),
        EventKind::SessionEnded => Some(match ev.draft.payload["outcome"].as_str() {
            Some("failed") | Some("error") | Some("gagal") => TaskPhase::Gagal,
            Some("batal") | Some("cancelled") => TaskPhase::Batal,
            _ => TaskPhase::Selesai,
        }),
        EventKind::LogRecorded => activity_sentence(agent, ev)
            .filter(|sentence| sentence.key == "agent.waiting_user")
            .map(|_| TaskPhase::MenungguRizal),
        _ => None,
    }
}

fn string_field(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_string)
}
