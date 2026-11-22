use crate::probes::ProcessInfo;
use crate::runtime::classify;
use crate::runtime::omp::{OmpMessageMeta, omp_status, slug};
use hq_core::{
    AgentId, AgentKind, EventDraft, EventKind, SessionId, SourceId, Subject, UsageScope,
};
use rusqlite::{Connection, OpenFlags, params};
use serde_json::json;
use std::path::Path;
use std::time::Duration;

const DAY_MS: i64 = 86_400_000;
const WINDOW_MS: i64 = 300_000;
const MAX_WORKERS: usize = 8;

pub struct OmpDbAdapter {
    path: std::path::PathBuf,
}

impl OmpDbAdapter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    fn connect(&self) -> Option<Connection> {
        let encoded = self
            .path
            .to_string_lossy()
            .replace('%', "%25")
            .replace('?', "%3f")
            .replace('#', "%23");
        let uri = format!("file:{encoded}?mode=ro");
        let conn = Connection::open_with_flags(
            uri,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .ok()?;
        conn.busy_timeout(Duration::from_secs(2)).ok()?;
        conn.execute_batch("PRAGMA query_only = ON;").ok()?;
        Some(conn)
    }

    pub fn observe_rows(&self, now_ms: i64, processes: &[&ProcessInfo]) -> Vec<EventDraft> {
        let process_count = processes.len();
        let cpu_percent = processes
            .iter()
            .map(|process| process.pcpu as f64)
            .sum::<f64>();
        let Some(conn) = self.connect() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        let since = now_ms.saturating_sub(DAY_MS);
        let sessions: Vec<SessionRow> = {
            let Ok(mut stmt) = conn.prepare(
                "SELECT session_file, requests, started_at, ended_at, total_tokens, cost_total,
                        tool_calls, models
                 FROM session_rollup WHERE ended_at > ?1 ORDER BY ended_at ASC LIMIT 500",
            ) else {
                return Vec::new();
            };
            let Ok(rows) = stmt.query_map(params![since], |row| {
                Ok(SessionRow {
                    file: row.get(0)?,
                    requests: row.get::<_, Option<i64>>(1)?.unwrap_or_default(),
                    started_ms: row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                    ended_ms: row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
                    tokens: row.get::<_, Option<i64>>(4)?.unwrap_or_default(),
                    cost: row.get::<_, Option<f64>>(5)?.unwrap_or_default(),
                    tool_calls: row.get::<_, Option<i64>>(6)?.unwrap_or_default(),
                    models: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                })
            }) else {
                return Vec::new();
            };
            rows.filter_map(Result::ok).collect()
        };
        let mut folder = String::from("unknown");
        for session in &sessions {
            let id = session_id(&session.file);
            let subject = Subject::Session(SessionId(format!("agent:omp/session/{id}")));
            let started_ms = session.started_ms.max(1);
            let ended_ms = session.ended_ms.max(1);
            events.push(EventDraft {
                event_ts_ms: started_ms,
                kind: EventKind::SessionStarted,
                subject: subject.clone(),
                source: SourceId("local".to_owned()),
                correlation_id: None,
                causation_id: None,
                idempotency_key: Some(format!("omp:db:session-started:{id}:{started_ms}")),
                payload: json!({
                    "started_at_ms": started_ms,
                    "requests": session.requests,
                    "tool_calls": session.tool_calls,
                    "models": session.models,
                    "evidence": "L3",
                    "source": "omp stats.db session_rollup",
                }),
            });
            events.push(EventDraft {
                event_ts_ms: ended_ms,
                kind: EventKind::SessionEnded,
                subject,
                source: SourceId("local".to_owned()),
                correlation_id: None,
                causation_id: None,
                idempotency_key: Some(format!("omp:db:session-ended:{id}:{ended_ms}")),
                payload: json!({
                    "ended_at_ms": ended_ms,
                    "requests": session.requests,
                    "evidence": "L3",
                    "source": "omp stats.db session_rollup",
                }),
            });
            events.push(EventDraft {
                event_ts_ms: ended_ms,
                kind: EventKind::UsageRecorded,
                subject: Subject::Usage(UsageScope::Session(SessionId(id.clone()))),
                source: SourceId("local".to_owned()),
                correlation_id: None,
                causation_id: None,
                idempotency_key: Some(format!("omp:db:usage:{id}:{ended_ms}")),
                payload: json!({
                    "tokens": session.tokens,
                    "requests": session.requests,
                    "cost_usd": session.cost,
                    "tool_calls": session.tool_calls,
                    "evidence": "L3",
                    "source": "omp stats.db session_rollup",
                }),
            });
            if folder == "unknown" {
                folder = session.file.clone();
            }
        }

        let workers: Vec<WorkerRow> = {
            let Ok(mut stmt) = conn.prepare(
                "SELECT agent_type, MAX(timestamp), SUM(tokens), SUM(cost),
                        MAX(stop_reason), MAX(error_message)
                 FROM messages
                 WHERE timestamp > ?1 AND agent_type IS NOT NULL AND agent_type <> ''
                   AND LOWER(agent_type) <> 'main'
                 GROUP BY agent_type ORDER BY SUM(tokens) DESC",
            ) else {
                return events;
            };
            let Ok(rows) = stmt.query_map(params![now_ms.saturating_sub(WINDOW_MS)], |row| {
                Ok(WorkerRow {
                    agent_type: row.get(0)?,
                    last_ms: row.get::<_, Option<i64>>(1)?.unwrap_or_default(),
                    tokens: row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                    cost: row.get::<_, Option<f64>>(3)?.unwrap_or_default(),
                    stop_reason: row.get::<_, Option<String>>(4)?,
                    error_message: row.get::<_, Option<String>>(5)?,
                })
            }) else {
                return events;
            };
            rows.filter_map(Result::ok).collect()
        };
        for worker in workers.iter().take(MAX_WORKERS) {
            let subject = Subject::Agent(AgentId(format!(
                "agent:omp/worker/{}",
                slug(&worker.agent_type)
            )));
            let observed = omp_status(
                processes.first().copied().cloned(),
                Some(OmpMessageMeta {
                    timestamp_ms: worker.last_ms,
                    stop_reason: worker.stop_reason.clone(),
                    error_message: worker.error_message.clone(),
                    folder: Some(session_id(&folder)),
                }),
                now_ms,
            );
            events.push(EventDraft {
                event_ts_ms: worker.last_ms.max(1),
                kind: EventKind::AgentObserved,
                subject,
                source: SourceId("local".to_owned()),
                correlation_id: None,
                causation_id: None,
                idempotency_key: Some(format!(
                    "omp:db:worker:{}:{}",
                    slug(&worker.agent_type),
                    worker.last_ms
                )),
                payload: json!({
                    "observed_at_ms": worker.last_ms.max(1),
                    "runtime_key": format!("omp:worker:{}", slug(&worker.agent_type)),
                    "status": observed.status,
                    "status_reason": observed.reason,
                    "kind": AgentKind::Worker,
                    "activity": observed.activity,
                    "processes": process_count as u32,
                    "cpu_percent": cpu_percent,
                    "evidence": observed.evidence,
                    "parent": "agent:omp",
                    "worker_type": worker.agent_type,
                    "tokens": worker.tokens,
                    "cost_usd": worker.cost,
                    "source": "omp stats.db messages",
                }),
            });
            events.push(EventDraft {
                event_ts_ms: worker.last_ms.max(1),
                kind: EventKind::UsageRecorded,
                subject: Subject::Usage(UsageScope::Agent(AgentId(format!(
                    "agent:omp/worker/{}",
                    slug(&worker.agent_type)
                )))),
                source: SourceId("local".to_owned()),
                correlation_id: None,
                causation_id: None,
                idempotency_key: Some(format!(
                    "omp:db:worker-usage:{}:{}",
                    slug(&worker.agent_type),
                    worker.last_ms
                )),
                payload: json!({
                    "tokens": worker.tokens,
                    "cost_usd": worker.cost,
                    "worker_type": worker.agent_type,
                    "evidence": "L3",
                    "source": "omp stats.db messages",
                }),
            });
        }
        if workers.len() > MAX_WORKERS {
            let tokens: i64 = workers.iter().map(|worker| worker.tokens).sum();
            let cost: f64 = workers.iter().map(|worker| worker.cost).sum();
            let last_ms = workers
                .iter()
                .map(|worker| worker.last_ms)
                .max()
                .unwrap_or(1);
            let observed = omp_status(
                processes.first().copied().cloned(),
                Some(OmpMessageMeta {
                    timestamp_ms: last_ms,
                    stop_reason: Some("toolUse".to_owned()),
                    error_message: None,
                    folder: Some(session_id(&folder)),
                }),
                now_ms,
            );
            events.push(EventDraft {
                event_ts_ms: last_ms.max(1),
                kind: EventKind::AgentObserved,
                subject: Subject::Agent(AgentId("agent:omp/worker/agg".to_owned())),
                source: SourceId("local".to_owned()),
                correlation_id: None,
                causation_id: None,
                idempotency_key: Some(format!("omp:db:workers-agg:{last_ms}")),
                payload: json!({
                    "observed_at_ms": last_ms.max(1),
                    "runtime_key": "omp:worker:agg",
                    "status": observed.status,
                    "status_reason": observed.reason,
                    "kind": AgentKind::Worker,
                    "activity": observed.activity,
                    "processes": process_count as u32,
                    "cpu_percent": cpu_percent,
                    "evidence": observed.evidence,
                    "parent": "agent:omp",
                    "worker_type": "omp-workers-agg",
                    "worker_types": workers.len(),
                    "tokens": tokens,
                    "cost_usd": cost,
                    "source": "omp stats.db messages",
                }),
            });
        }
        events
    }
}

#[async_trait::async_trait]
impl crate::runtime::RuntimeAdapter for OmpDbAdapter {
    async fn observe(&self, ctx: &crate::runtime::CollectCtx) -> Result<Vec<EventDraft>, String> {
        let omp: Vec<&ProcessInfo> = ctx
            .processes
            .iter()
            .filter(|process| classify(process) == Some("omp"))
            .collect();
        Ok(self.observe_rows(ctx.now_ms, &omp))
    }
}

struct SessionRow {
    file: String,
    requests: i64,
    started_ms: i64,
    ended_ms: i64,
    tokens: i64,
    cost: f64,
    tool_calls: i64,
    models: String,
}

struct WorkerRow {
    agent_type: String,
    last_ms: i64,
    tokens: i64,
    cost: f64,
    stop_reason: Option<String>,
    error_message: Option<String>,
}

fn session_id(session_file: &str) -> String {
    Path::new(session_file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(session_file)
        .to_owned()
}
