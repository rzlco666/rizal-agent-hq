use crate::probes::{ProcessInfo, run_timeout};
use hq_core::{
    AgentId, AgentKind, EventDraft, EventKind, EvidenceLevel, Sentence, SourceId, Status, Subject,
    UsageScope, redact_text,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const OMP_STATS_TIMEOUT_MS: u64 = 20_000;
pub const OMP_STATS_CACHE_MS: u64 = 300_000;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpStats {
    pub overall: OmpOverall,
    pub by_model: Vec<OmpModel>,
    pub by_folder: Vec<OmpFolder>,
    pub by_agent_type: Vec<OmpAgentType>,
    pub time_series: Vec<OmpPoint>,
    pub cost_series: Vec<OmpCostPoint>,
    pub quota: Option<OmpQuota>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpOverall {
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cost: f64,
    pub error_rate: f64,
    pub last_timestamp: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpModel {
    pub provider: String,
    pub model: String,
    pub total_cost: f64,
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpFolder {
    pub folder: String,
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpAgentType {
    pub agent_type: String,
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpPoint {
    pub timestamp: i64,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpCostPoint {
    pub timestamp: i64,
    pub cost: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OmpQuota {
    pub provider: String,
    pub label: String,
    pub percent: Option<f64>,
    pub resets_ms: Option<i64>,
}

pub struct OmpAdapter {
    home: PathBuf,
    program: String,
    cache: Mutex<Option<(Instant, OmpStats)>>,
}

impl Default for OmpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OmpAdapter {
    pub fn new() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        let program = std::env::var_os("HQ_OMP_STATS_BIN")
            .map(|path| PathBuf::from(path).to_string_lossy().into_owned())
            .unwrap_or_else(|| "omp".to_owned());
        let mut adapter = Self::with_home(home);
        adapter.program = program;
        adapter
    }
    pub fn with_home(home: PathBuf) -> Self {
        Self {
            home,
            program: "omp".to_owned(),
            cache: Mutex::new(None),
        }
    }

    /// Inject an executable for tests; it still receives `stats --json`.
    pub fn with_command(program: impl Into<String>) -> Self {
        Self {
            home: PathBuf::from("/"),
            program: program.into(),
            cache: Mutex::new(None),
        }
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    fn cached(&self) -> Option<OmpStats> {
        let guard = self.cache.lock().ok()?;
        let (fetched, stats) = guard.as_ref()?;
        Instant::now()
            .checked_duration_since(*fetched)
            .is_some_and(|age| age < Duration::from_millis(OMP_STATS_CACHE_MS))
            .then(|| stats.clone())
    }

    pub fn stats(&self) -> Option<OmpStats> {
        if let Some(stats) = self.cached() {
            return Some(stats);
        }
        let output = run_timeout(&self.program, &["stats", "--json"], OMP_STATS_TIMEOUT_MS)?;
        if !output.success {
            return self.cached();
        }
        let stats: OmpStats = serde_json::from_str(&output.stdout).ok()?;
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some((Instant::now(), stats.clone()));
        }
        Some(stats)
    }

    pub fn observe_stats(&self, now_ms: i64) -> Vec<EventDraft> {
        self.stats()
            .map(|stats| omp_usage_events(&stats, now_ms))
            .unwrap_or_default()
    }

    /// Refresh path for one-shot collectors: run the bounded command only when
    /// no fresh cached stats exist.
    pub fn refresh_if_cold(&self) -> bool {
        if self.cached().is_some() {
            return false;
        }
        self.stats().is_some()
    }

    /// Snapshot path: never runs a command, only emits fresh cached stats.
    pub fn observe_cached(&self, ctx: &crate::runtime::CollectCtx) -> Vec<EventDraft> {
        self.cached()
            .map(|stats| events_from_stats(&stats, ctx.now_ms, &ctx.processes))
            .unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl crate::runtime::RuntimeAdapter for OmpAdapter {
    async fn observe(&self, ctx: &crate::runtime::CollectCtx) -> Result<Vec<EventDraft>, String> {
        Ok(self.observe_cached(ctx))
    }
}

fn events_from_stats(stats: &OmpStats, now_ms: i64, processes: &[ProcessInfo]) -> Vec<EventDraft> {
    let mut events = omp_usage_events(stats, now_ms);
    let omp: Vec<&ProcessInfo> = processes
        .iter()
        .filter(|process| crate::runtime::classify(process) == Some("omp"))
        .collect();
    let latest = (stats.overall.last_timestamp > 0).then(|| OmpMessageMeta {
        timestamp_ms: stats.overall.last_timestamp,
        stop_reason: None,
        error_message: None,
        folder: stats.by_folder.first().map(|folder| folder.folder.clone()),
    });
    let activity = omp_status(omp.first().copied().cloned(), latest, now_ms);
    events.insert(0, omp_agent_event(&omp, activity, now_ms));
    events
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmpMessageMeta {
    pub timestamp_ms: i64,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
    pub folder: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmpActivity {
    pub status: Status,
    pub activity: Sentence,
    pub evidence: EvidenceLevel,
    pub reason: &'static str,
}

/// Status comes only from process presence and message metadata. Message text is
/// never read; the folder is redacted because paths can carry tokens.
pub fn omp_status(
    process: Option<ProcessInfo>,
    latest: Option<OmpMessageMeta>,
    now_ms: i64,
) -> OmpActivity {
    let folder = latest
        .as_ref()
        .and_then(|message| message.folder.as_deref())
        .unwrap_or("unknown");
    let mut params = BTreeMap::new();
    params.insert(
        "folder".to_owned(),
        redact_text(folder).chars().take(200).collect(),
    );
    let activity = Sentence {
        key: "agent.working_in_folder".to_owned(),
        params,
    };
    let (status, reason) = if process.is_none() {
        (Status::Offline, "process_absent")
    } else {
        let age_ms = latest
            .as_ref()
            .map(|message| now_ms.saturating_sub(message.timestamp_ms).max(0))
            .unwrap_or(i64::MAX);
        let recent_error = latest.as_ref().is_some_and(|message| {
            age_ms < 300_000
                && message
                    .error_message
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
        });
        let tool_use = latest.as_ref().is_some_and(|message| {
            age_ms < 90_000
                && message.stop_reason.as_deref().is_some_and(|reason| {
                    reason.eq_ignore_ascii_case("tooluse")
                        || reason.eq_ignore_ascii_case("tool_use")
                })
        });
        if recent_error {
            (Status::Error, "recent_error")
        } else if tool_use {
            (Status::Bekerja, "tool_use")
        } else if age_ms < 600_000 {
            (Status::Berpikir, "recent_message")
        } else {
            (Status::Santai, "idle")
        }
    };
    OmpActivity {
        status,
        activity,
        evidence: EvidenceLevel::L3,
        reason,
    }
}

fn omp_agent_event(processes: &[&ProcessInfo], observed: OmpActivity, now_ms: i64) -> EventDraft {
    EventDraft {
        event_ts_ms: now_ms,
        kind: EventKind::AgentObserved,
        subject: Subject::Agent(AgentId("agent:omp".to_owned())),
        source: SourceId("local".to_owned()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(format!("omp:observed:{}", now_ms / 60_000)),
        payload: json!({
            "observed_at_ms": now_ms,
            "runtime_key": "omp:main",
            "status": observed.status,
            "status_reason": observed.reason,
            "kind": AgentKind::Utama,
            "activity": observed.activity,
            "processes": processes.len() as u32,
            "cpu_percent": processes.iter().map(|process| process.pcpu as f64).sum::<f64>(),
            "evidence": observed.evidence,
            "source": "omp",
        }),
    }
}

pub fn omp_usage_events(stats: &OmpStats, now_ms: i64) -> Vec<EventDraft> {
    let mut events = Vec::new();
    let overall = &stats.overall;
    events.push(usage_draft(
        now_ms,
        Subject::Usage(UsageScope::Global),
        "omp:stats:global",
        json!({
            "scope": "global",
            "provider": "all",
            "model": "all",
            "tokens": overall.total_input_tokens + overall.total_output_tokens + overall.total_cache_read_tokens,
            "input_tokens": overall.total_input_tokens,
            "output_tokens": overall.total_output_tokens,
            "cache_tokens": overall.total_cache_read_tokens,
            "requests": overall.total_requests,
            "cost_usd": overall.total_cost,
            "evidence": "L3",
            "source": "omp stats --json",
        }),
    ));
    for model in &stats.by_model {
        events.push(usage_draft(
            now_ms,
            Subject::Usage(UsageScope::Agent(AgentId("agent:omp".to_owned()))),
            format!(
                "omp:stats:model:{}:{}",
                slug(&model.provider),
                slug(&model.model)
            ),
            json!({
                "scope": "model",
                "provider": model.provider,
                "model": model.model,
                "tokens": model.total_input_tokens + model.total_output_tokens + model.total_cache_read_tokens,
                "input_tokens": model.total_input_tokens,
                "output_tokens": model.total_output_tokens,
                "cache_tokens": model.total_cache_read_tokens,
                "requests": model.total_requests,
                "cost_usd": model.total_cost,
                "evidence": "L3",
                "source": "omp stats --json",
            }),
        ));
    }
    for agent_type in &stats.by_agent_type {
        let is_main = matches!(agent_type.agent_type.as_str(), "" | "main");
        let subject = if is_main {
            Subject::Agent(AgentId("agent:omp".to_owned()))
        } else {
            Subject::Agent(AgentId(format!(
                "agent:omp/worker/{}",
                slug(&agent_type.agent_type)
            )))
        };
        events.push(usage_draft(
            now_ms,
            subject,
            format!("omp:stats:agent:{}", slug(&agent_type.agent_type)),
            json!({
                "scope": "agent_type",
                "agent_type": agent_type.agent_type,
                "worker": !is_main,
                "tokens": agent_type.total_input_tokens + agent_type.total_output_tokens,
                "input_tokens": agent_type.total_input_tokens,
                "output_tokens": agent_type.total_output_tokens,
                "requests": agent_type.total_requests,
                "cost_usd": agent_type.total_cost,
                "evidence": "L3",
                "source": "omp stats --json",
            }),
        ));
    }
    if let Some(quota) = &stats.quota {
        events.push(EventDraft {
            event_ts_ms: now_ms,
            kind: EventKind::QuotaObserved,
            subject: Subject::Usage(UsageScope::Agent(AgentId("agent:omp".to_owned()))),
            source: SourceId("local".to_owned()),
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(format!("omp:quota:{}", slug(&quota.provider))),
            payload: json!({
                "provider": quota.provider,
                "label": quota.label,
                "percent": quota.percent,
                "remaining": quota.percent.map(|p| (p * 100.0) as i64).unwrap_or_default(),
                "resets_ms": quota.resets_ms,
                "evidence": "L3",
                "source": "omp stats --json",
            }),
        });
    }
    events
}

fn usage_draft(
    now_ms: i64,
    subject: Subject,
    key: impl Into<String>,
    payload: serde_json::Value,
) -> EventDraft {
    EventDraft {
        event_ts_ms: now_ms,
        kind: EventKind::UsageRecorded,
        subject,
        source: SourceId("local".to_owned()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(key.into()),
        payload,
    }
}

pub(crate) fn slug(value: &str) -> String {
    let mut out = String::new();
    let mut dashed = false;
    for ch in value.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            dashed = false;
        } else if !dashed && !out.is_empty() {
            out.push('-');
            dashed = true;
        }
    }
    out.trim_matches('-').to_owned()
}
