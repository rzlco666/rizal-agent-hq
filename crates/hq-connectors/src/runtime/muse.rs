use crate::runtime::omp::slug;
use hq_core::{
    AgentId, AgentKind, EventDraft, EventKind, EvidenceLevel, Sentence, SourceId, Status, Subject,
    redact_text,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const FRESH_MS: i64 = 600_000;
const MAX_HEARTBEAT_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Heartbeat {
    pub agent: String,
    pub at: i64,
    pub role: String,
    pub status: String,
    pub task: String,
}

pub fn parse_heartbeat(input: &str) -> Option<Heartbeat> {
    let heartbeat: Heartbeat = serde_json::from_str(input).ok()?;
    (!heartbeat.agent.trim().is_empty() && heartbeat.at > 0).then_some(heartbeat)
}

pub struct MuseAdapter {
    data_dir: PathBuf,
}

impl MuseAdapter {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    pub fn observe_dirs(&self, now_ms: i64) -> Vec<EventDraft> {
        let mut latest: BTreeMap<String, Heartbeat> = BTreeMap::new();
        let Ok(entries) = fs::read_dir(self.data_dir.join("heartbeats")) else {
            return Vec::new();
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            if metadata.len() > MAX_HEARTBEAT_BYTES {
                continue;
            }
            let Ok(contents) = fs::read_to_string(&path) else {
                continue;
            };
            let Some(heartbeat) = parse_heartbeat(&contents) else {
                continue;
            };
            let agent = slug(&heartbeat.agent);
            let replace = latest
                .get(&agent)
                .is_none_or(|previous| heartbeat.at > previous.at);
            if replace {
                latest.insert(agent, heartbeat);
            }
        }

        latest
            .into_iter()
            .map(|(agent, heartbeat)| heartbeat_event(now_ms, &agent, &heartbeat))
            .collect()
    }
}

#[async_trait::async_trait]
impl crate::runtime::RuntimeAdapter for MuseAdapter {
    async fn observe(&self, ctx: &crate::runtime::CollectCtx) -> Result<Vec<EventDraft>, String> {
        Ok(self.observe_dirs(ctx.now_ms))
    }
}

fn heartbeat_event(now_ms: i64, agent: &str, heartbeat: &Heartbeat) -> EventDraft {
    let age_ms = now_ms.saturating_sub(heartbeat.at).max(0);
    let status = if age_ms < FRESH_MS {
        heartbeat_status(&heartbeat.status)
    } else {
        Status::Offline
    };
    let mut params = BTreeMap::new();
    params.insert(
        "text".to_owned(),
        redact_text(&heartbeat.task).chars().take(200).collect(),
    );
    let activity = Sentence {
        key: "agent.heartbeat_activity".to_owned(),
        params,
    };
    EventDraft {
        event_ts_ms: heartbeat.at,
        kind: EventKind::AgentObserved,
        subject: Subject::Agent(AgentId(format!("agent:{agent}"))),
        source: SourceId("local".to_owned()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(format!("muse:heartbeat:{agent}:{}", heartbeat.at)),
        payload: json!({
            "observed_at_ms": heartbeat.at,
            "runtime_key": format!("muse:heartbeat:{agent}"),
            "status": status,
            "kind": AgentKind::Utama,
            "activity": activity,
            "processes": 0_u32,
            "cpu_percent": 0.0,
            "evidence": EvidenceLevel::L2,
            "heartbeat_age_ms": age_ms,
            "role": heartbeat.role,
            "source": "muse heartbeat v2",
        }),
    }
}

fn heartbeat_status(status: &str) -> Status {
    match status.trim().to_ascii_lowercase().as_str() {
        "bekerja" | "working" | "active" => Status::Bekerja,
        "berpikir" | "thinking" => Status::Berpikir,
        "menunggu" | "menunggu-rizal" | "waiting" => Status::MenungguRizal,
        "error" | "failed" => Status::Error,
        _ => Status::Santai,
    }
}
