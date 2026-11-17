#![forbid(unsafe_code)]

pub mod auth;
pub mod error;
pub mod ingest;
pub mod projections;
pub mod reads;
pub mod ring;

use hq_humanize::render;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{broadcast, oneshot};

pub use auth::{DeviceRecord, PairingRecord};
pub use error::StoreError;
pub use hq_core::*;
pub use hq_humanize::{Locale, LogLevel, TaskDeriver, TaskProjection, TaskStepProjection};
pub use ingest::{IngestReport, Ingestor};
pub use projections::Projector;
pub use reads::{AlertView as StoreAlertView, ChangedFile, CostSummary, SessionView, TaskView};
pub use ring::PatchRing;
// ---------- Options ----------
#[derive(Clone, Debug)]
pub struct StoreOptions {
    pub path: PathBuf,
    pub read_pool: usize,
    pub busy_timeout_ms: u64,
}
impl StoreOptions {
    pub fn for_dir(dir: &Path) -> Self {
        Self {
            path: dir.join("hq.db"),
            read_pool: 4,
            busy_timeout_ms: 5000,
        }
    }
    pub fn at_file(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            read_pool: 2,
            busy_timeout_ms: 5000,
        }
    }
}

// ---------- Health ----------
#[derive(Debug, Clone)]
pub struct Health {
    pub journal_mode: String,
    pub tables: Vec<String>,
    pub migrations: i64,
    pub wal_size_bytes: u64,
    pub last_seq: i64,
    pub integrity_ok: bool,
}

// ---------- Overview ----------
#[derive(Debug, Clone, Default)]
pub struct Overview {
    pub agents: i64,
    pub alerts_open: i64,
    pub usage_requests: i64,
    pub usage_input_tokens: i64,
}

#[derive(Debug, Clone, Default)]
pub struct SystemHealthReport {
    pub latest: Option<projections::system_health::SystemHealthSample>,
    pub history: Vec<projections::system_health::SystemHealthSample>,
}

// ---------- Usage ----------
#[derive(Debug, Clone)]
pub struct UsageTotals {
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_tokens: i64,
    pub cash_usd: f64,
    pub shadow_usd: f64,
}
#[derive(Debug, Clone)]
pub struct UsageReport {
    pub total: UsageTotals,
}
#[derive(Debug, Clone)]
pub struct UsageQuery {
    pub agent: Option<String>,
    pub kind: String,
}
impl UsageQuery {
    pub fn agent_minutes(s: &str) -> Self {
        Self {
            agent: Some(s.to_string()),
            kind: "minutes".to_string(),
        }
    }
}

// ---------- Agent read model ----------
#[derive(Debug, Clone)]
pub struct AgentCard {
    pub id: String,
    pub name: String,
    pub status: Status,
}
#[derive(Debug, Clone)]
pub struct AgentDetail {
    pub card: AgentCard,
}

// ---------- Log read model ----------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub id: String,
    pub ts_ms: i64,
    pub level: LogLevel,
    pub text: String,
}

// ---------- Patch ----------
#[derive(Debug, Clone)]
pub struct PatchRef {
    pub kind: String,
    pub id: String,
}
#[derive(Debug, Clone)]
pub enum PatchEntity {
    Agent(String, EventKind, serde_json::Value),
    Task(String, EventKind, serde_json::Value),
    Log(String, EventKind, serde_json::Value),
    Alert(String, EventKind, serde_json::Value),
    Usage(String, EventKind, serde_json::Value),
    Connector(String, EventKind, serde_json::Value),
    Approval(String, EventKind, serde_json::Value),
}
#[derive(Debug, Clone)]
pub struct Patch {
    pub seq: Seq,
    pub upserts: Vec<PatchEntity>,
    pub deletes: Vec<PatchRef>,
}

pub fn patch_for_event(ev: &Event) -> Patch {
    use hq_core::EventKind;

    let id = match &ev.draft.subject {
        Subject::Usage(UsageScope::Session(session)) => session.0.clone(),
        subject => subject.key(),
    };
    let entity_type = match ev.draft.kind {
        EventKind::AgentObserved
        | EventKind::AgentSeen
        | EventKind::AgentStatusChanged
        | EventKind::AgentDisappeared => "agent",
        EventKind::SessionStarted | EventKind::SessionUpdated | EventKind::SessionEnded => "task",
        EventKind::LogRecorded => "log",
        EventKind::AlertRaised
        | EventKind::AlertUpdated
        | EventKind::AlertAcknowledged
        | EventKind::AlertSnoozed
        | EventKind::AlertResolved => "alert",
        EventKind::UsageRecorded | EventKind::QuotaObserved | EventKind::CostEstimated => "usage",
        EventKind::ConnectorRegistered
        | EventKind::ConnectorHealthChanged
        | EventKind::ConnectorPollCompleted
        | EventKind::ConnectorAuthFailed
        | EventKind::ConnectorEvent => "connector",
        EventKind::ActionRequested
        | EventKind::ActionApproved
        | EventKind::ActionDenied
        | EventKind::ActionExecuted
        | EventKind::ActionFailed => "approval",
        _ => "",
    };

    let mut upserts = Vec::new();
    if !entity_type.is_empty() {
        let mut payload = ev.draft.payload.clone();
        hq_core::redact_json(&mut payload);
        let delta = match entity_type {
            "agent" => PatchEntity::Agent(id, ev.draft.kind, payload),
            "task" => PatchEntity::Task(id, ev.draft.kind, payload),
            "log" => PatchEntity::Log(id, ev.draft.kind, payload),
            "alert" => PatchEntity::Alert(id, ev.draft.kind, payload),
            "usage" => PatchEntity::Usage(id, ev.draft.kind, payload),
            "connector" => PatchEntity::Connector(id, ev.draft.kind, payload),
            "approval" => PatchEntity::Approval(id, ev.draft.kind, payload),
            _ => unreachable!(),
        };
        upserts.push(delta);
    }
    Patch {
        seq: ev.seq,
        upserts,
        deletes: Vec::new(),
    }
}

// ---------- Event query ----------
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub kind: Option<String>,
    pub subject: Option<String>,
    pub since: Option<i64>,
    pub until: Option<i64>,
    pub limit: Option<usize>,
    pub cursor: Option<i64>,
}
#[derive(Debug, Clone)]
pub struct EventPage {
    pub events: Vec<Event>,
    pub next_cursor: Option<i64>,
}

// ---------- Rebuild ----------
#[derive(Debug, Clone)]
pub enum RebuildOptions {
    Shadow(PathBuf),
    InPlaceTest,
}
impl RebuildOptions {
    pub fn shadow(p: PathBuf) -> Self {
        Self::Shadow(p)
    }
    pub fn in_place_test() -> Self {
        Self::InPlaceTest
    }
}
#[derive(Debug, Clone)]
pub struct RebuildReport {
    pub events: usize,
    pub duration_ms: u64,
}

// ---------- StoreInner ----------
struct StoreInner {
    opts: StoreOptions,
    writer_tx: std::sync::mpsc::Sender<WriteCmd>,
    read_pool: Pool<SqliteConnectionManager>,
    broadcast: broadcast::Sender<Patch>,
    #[cfg(any(test, debug_assertions))]
    fail_after_insert: Arc<AtomicBool>,
    #[cfg(any(test, debug_assertions))]
    fail_last_seq: Arc<AtomicBool>,
    #[cfg(any(test, debug_assertions))]
    fail_read_since: Arc<AtomicBool>,
    integrity_ok: Arc<AtomicBool>,
}
enum WriteCmd {
    Append {
        drafts: Vec<EventDraft>,
        #[cfg(any(test, debug_assertions))]
        fail_after_insert: bool,
        reply: oneshot::Sender<Result<Vec<Event>, StoreError>>,
    },
    Rebuild {
        opts: RebuildOptions,
        db_path: PathBuf,
        reply: oneshot::Sender<Result<RebuildReport, StoreError>>,
    },
    AuthWrite {
        op: crate::auth::AuthWrite,
        reply: oneshot::Sender<Result<Option<crate::auth::PairingRecord>, StoreError>>,
    },
}

#[derive(Clone)]
pub struct Store {
    inner: Arc<StoreInner>,
}

fn run_migrations(conn: &mut Connection) -> Result<(), StoreError> {
    let sql = include_str!("../migrations/0001_init.sql");
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_migrations'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if table_exists == 0 {
        conn.execute_batch(sql)?;
    }
    let tx = conn.transaction()?;
    let mut hasher = Sha256::new();
    hasher.update(sql.as_bytes());
    let checksum = format!("{:x}", hasher.finalize());
    let exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM _migrations WHERE version=1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        tx.execute(
            "INSERT INTO _migrations (version, name, checksum) VALUES (1, '0001_init.sql', ?1)",
            params![checksum],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn check_integrity(conn: &Connection) -> bool {
    let r: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap_or_else(|_| "error".to_string());
    r == "ok"
}

pub fn agent_name(id: &str) -> String {
    id.split(':').next_back().unwrap_or(id).to_uppercase()
}

pub fn status_from_str(s: &str) -> Status {
    match s {
        "bekerja" => Status::Bekerja,
        "berpikir" => Status::Berpikir,
        "menunggu-rizal" => Status::MenungguRizal,
        "santai" => Status::Santai,
        "offline" => Status::Offline,
        "error" => Status::Error,
        _ => Status::Bekerja,
    }
}

fn make_projectors() -> Vec<Box<dyn projections::Projector>> {
    vec![
        Box::new(projections::agents::AgentProjector),
        Box::new(projections::logs::LogProjector),
        Box::new(projections::tasks::TaskProjector),
        Box::new(projections::usage::UsageProjector),
        Box::new(projections::alerts::AlertProjector),
        Box::new(projections::system_health::SystemHealthProjector),
    ]
}

impl Store {
    pub async fn open(opts: StoreOptions) -> Result<Self, StoreError> {
        let opts2 = opts.clone();
        let (txw, rxw) = std::sync::mpsc::channel::<WriteCmd>();
        #[cfg(any(test, debug_assertions))]
        let fail_flag = Arc::new(AtomicBool::new(false));
        #[cfg(any(test, debug_assertions))]
        let fail_flag_outer = fail_flag.clone();
        #[cfg(any(test, debug_assertions))]
        let fail_last_seq = Arc::new(AtomicBool::new(false));
        #[cfg(any(test, debug_assertions))]
        let fail_read_since = Arc::new(AtomicBool::new(false));
        let integrity_flag = Arc::new(AtomicBool::new(false));
        let integrity_flag_outer = integrity_flag.clone();
        let (broadcast_tx, _) = broadcast::channel(1024);
        let broadcast_for_thread = broadcast_tx.clone();
        let projectors = make_projectors();

        let pool = tokio::task::spawn_blocking(
            move || -> Result<Pool<SqliteConnectionManager>, StoreError> {
                if let Some(parent) = opts2.path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut conn = Connection::open(&opts2.path)?;
                conn.execute_batch("PRAGMA journal_mode=WAL;")?;
                conn.execute_batch(&format!("PRAGMA busy_timeout={};", opts2.busy_timeout_ms))?;
                conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
                conn.execute_batch("PRAGMA foreign_keys=ON;")?;
                run_migrations(&mut conn)?;
                let ok = check_integrity(&conn);
                let opts_t = opts2.clone();
                let bc = broadcast_for_thread.clone();
                let projs = projectors;
                std::thread::spawn(move || {
                    writer_loop(conn, rxw, opts_t, bc, projs);
                });
                let manager = SqliteConnectionManager::file(&opts2.path).with_init(move |c| {
                    c.execute_batch("PRAGMA query_only=ON;")?;
                    c.execute_batch(&format!("PRAGMA busy_timeout={};", opts2.busy_timeout_ms))?;
                    Ok(())
                });
                let pool = Pool::new(manager).map_err(|e| StoreError::Pool(e.to_string()))?;
                integrity_flag.store(ok, Ordering::SeqCst);
                Ok(pool)
            },
        )
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))??;

        let inner = Arc::new(StoreInner {
            opts,
            writer_tx: txw,
            read_pool: pool,
            broadcast: broadcast_tx,
            #[cfg(any(test, debug_assertions))]
            fail_after_insert: fail_flag_outer.clone(),
            #[cfg(any(test, debug_assertions))]
            fail_last_seq,
            #[cfg(any(test, debug_assertions))]
            fail_read_since,
            integrity_ok: integrity_flag_outer,
        });
        Ok(Self { inner })
    }

    #[cfg(any(test, debug_assertions))]
    pub fn test_fail_after_insert(&self, v: bool) {
        self.inner.fail_after_insert.store(v, Ordering::SeqCst);
    }

    #[cfg(any(test, debug_assertions))]
    pub fn test_fail_reads(&self, v: bool) {
        self.inner.fail_read_since.store(v, Ordering::SeqCst);
    }

    #[cfg(any(test, debug_assertions))]
    pub fn test_fail_last_seq(&self, v: bool) {
        self.inner.fail_last_seq.store(v, Ordering::SeqCst);
    }

    pub async fn append(&self, drafts: Vec<EventDraft>) -> Result<Vec<Event>, StoreError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        #[cfg(any(test, debug_assertions))]
        let fail = self.inner.fail_after_insert.load(Ordering::SeqCst);
        let cmd = WriteCmd::Append {
            drafts,
            #[cfg(any(test, debug_assertions))]
            fail_after_insert: fail,
            reply: reply_tx,
        };
        self.inner
            .writer_tx
            .send(cmd)
            .map_err(|_| StoreError::ChannelClosed)?;
        reply_rx.await.map_err(|_| StoreError::ChannelClosed)?
    }

    /// Render stored sentence key plus params in the caller's locale.
    pub async fn logs(
        &self,
        agent: &AgentId,
        locale: Locale,
        limit: usize,
    ) -> Result<Vec<LogEntry>, StoreError> {
        let pool = self.inner.read_pool.clone();
        let agent = agent.0.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<LogEntry>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut stmt = conn.prepare(
                "SELECT id, ts_ms, level, sentence_json FROM logs
                 WHERE agent_id=?1 ORDER BY ts_ms DESC, id DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![agent, limit as i64], |r| {
                let id: String = r.get(0)?;
                let ts_ms: i64 = r.get(1)?;
                let level: String = r.get(2)?;
                let json: String = r.get(3)?;
                let sentence: Sentence = serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(LogEntry {
                    id,
                    ts_ms,
                    level: LogLevel::parse(&level),
                    text: render(&sentence, locale),
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn count_events(&self) -> Result<usize, StoreError> {
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<usize, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let n: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
            Ok(n as usize)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn last_seq(&self) -> Result<Seq, StoreError> {
        #[cfg(any(test, debug_assertions))]
        if self.inner.fail_last_seq.load(Ordering::SeqCst) {
            return Err(StoreError::Pool("injected last_seq failure".to_string()));
        }
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Seq, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let n: i64 =
                conn.query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
            // include archive max
            let a: i64 =
                conn.query_row("SELECT COALESCE(MAX(seq),0) FROM events_archive", [], |r| {
                    r.get(0)
                })?;
            Ok(Seq(n.max(a)))
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn agents(&self) -> Result<Vec<AgentDetail>, StoreError> {
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<AgentDetail>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut stmt = conn.prepare("SELECT id, name, status FROM agents")?;
            let rows = stmt.query_map([], |r| {
                Ok(AgentDetail {
                    card: AgentCard {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        status: status_from_str(&r.get::<_, String>(2)?),
                    },
                })
            })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn agent(
        &self,
        id: &AgentId,
        as_of: Option<Ms>,
    ) -> Result<Option<AgentDetail>, StoreError> {
        if let Some(asof) = as_of {
            // replay per agent from events with event_ts_ms <= asof
            let pool = self.inner.read_pool.clone();
            let idstr = id.0.clone();
            let asof: i64 = asof;
            return tokio::task::spawn_blocking(move || -> Result<Option<AgentDetail>, StoreError> {
                let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
                // collect events for this subject ordered seq
                let mut stmt = conn.prepare(
                    "SELECT kind,payload,event_ts_ms FROM events WHERE subject=?1 AND event_ts_ms <= ?2 ORDER BY seq",
                )?;
                let rows = stmt.query_map(params![idstr.clone(), asof], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
                })?;
                let mut status = Status::Bekerja;
                let mut seen = false;
                for r in rows {
                    let (kind, payload, _ts) = r?;
                    if kind.starts_with("agent_") {
                        seen = true;
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload)
                            && let Some(s) = v.get("status").and_then(|x| x.as_str()) {
                            status = status_from_str(s);
                        }
                    }
                }
                // archive
                let mut stmt2 = conn.prepare(
                    "SELECT kind,payload,event_ts_ms FROM events_archive WHERE subject=?1 AND event_ts_ms <= ?2 ORDER BY seq",
                )?;
                let rows2 = stmt2.query_map(params![idstr.clone(), asof], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
                })?;
                for r in rows2 {
                    let (kind, payload, _ts) = r?;
                    if kind.starts_with("agent_") {
                        seen = true;
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload)
                            && let Some(s) = v.get("status").and_then(|x| x.as_str()) {
                            status = status_from_str(s);
                        }
                    }
                }
                if !seen {
                    return Ok(None);
                }
                Ok(Some(AgentDetail {
                    card: AgentCard {
                        id: idstr.clone(),
                        name: agent_name(&idstr),
                        status,
                    },
                }))
            })
            .await
            .map_err(|e| StoreError::Pool(e.to_string()))?;
        }
        // normal read
        let pool = self.inner.read_pool.clone();
        let id = id.0.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<AgentDetail>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut stmt = conn.prepare("SELECT id, name, status FROM agents WHERE id=?1")?;
            let mut rows = stmt.query(params![id])?;
            if let Some(r) = rows.next()? {
                let id: String = r.get(0)?;
                let name: String = r.get(1)?;
                let status: String = r.get(2)?;
                Ok(Some(AgentDetail {
                    card: AgentCard {
                        id,
                        name,
                        status: status_from_str(&status),
                    },
                }))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn usage(&self, q: UsageQuery) -> Result<UsageReport, StoreError> {
        let pool = self.inner.read_pool.clone();
        let agent = q.agent.clone();
        tokio::task::spawn_blocking(move || -> Result<UsageReport, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let sql = if agent.is_some() {
                "SELECT COALESCE(SUM(requests),0),COALESCE(SUM(input_tokens),0),COALESCE(SUM(output_tokens),0),COALESCE(SUM(cache_tokens),0),COALESCE(SUM(cash_usd),0),COALESCE(SUM(shadow_usd),0) FROM usage_minute WHERE agent_id=?1"
            } else {
                "SELECT COALESCE(SUM(requests),0),COALESCE(SUM(input_tokens),0),COALESCE(SUM(output_tokens),0),COALESCE(SUM(cache_tokens),0),COALESCE(SUM(cash_usd),0),COALESCE(SUM(shadow_usd),0) FROM usage_minute"
            };
            let (requests, input, output, cache, cash, shadow): (i64, i64, i64, i64, f64, f64) =
                if let Some(a) = agent {
                    conn.query_row(sql, params![a], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
                    })?
                } else {
                    conn.query_row(sql, [], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
                    })?
                };
            Ok(UsageReport {
                total: UsageTotals {
                    requests,
                    input_tokens: input,
                    output_tokens: output,
                    cache_tokens: cache,
                    cash_usd: cash,
                    shadow_usd: shadow,
                },
            })
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub fn patches(&self) -> broadcast::Receiver<Patch> {
        self.inner.broadcast.subscribe()
    }

    pub async fn read_since(&self, since: Seq, limit: usize) -> Result<Vec<Event>, StoreError> {
        #[cfg(any(test, debug_assertions))]
        if self.inner.fail_read_since.load(Ordering::SeqCst) {
            return Err(StoreError::Pool("injected read_since failure".to_string()));
        }
        let pool = self.inner.read_pool.clone();
        let since = since.0;
        let limit = limit.min(1000);
        tokio::task::spawn_blocking(move || -> Result<Vec<Event>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut stmt = conn.prepare(
                "SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,correlation_id,causation_id,idempotency_key,payload \
                 FROM events WHERE seq > ?1 \
                 UNION ALL \
                 SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,correlation_id,causation_id,idempotency_key,payload \
                 FROM events_archive WHERE seq > ?1 \
                 ORDER BY seq LIMIT ?2",
            )?;
            let mut rows = stmt.query(params![since, limit as i64])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                out.push(row_to_event(row)?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn query_events(&self, q: EventQuery) -> Result<EventPage, StoreError> {
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<EventPage, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let limit = q.limit.unwrap_or(100).min(1000);
            let cursor = q.cursor.unwrap_or(0);

            // collect from events + archive
            let mut out = Vec::new();
            for table in ["events", "events_archive"] {
                let mut sql = format!(
                    "SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,correlation_id,causation_id,idempotency_key,payload FROM {} WHERE seq > ?1",
                    table
                );
                // filters
                let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(cursor)];
                if let Some(k) = &q.kind {
                    sql.push_str(" AND kind = ?");
                    params_vec.push(Box::new(k.clone()));
                }
                if let Some(s) = &q.subject {
                    sql.push_str(" AND subject = ?");
                    params_vec.push(Box::new(s.clone()));
                }
                if let Some(since) = q.since {
                    sql.push_str(" AND event_ts_ms >= ?");
                    params_vec.push(Box::new(since));
                }
                if let Some(until) = q.until {
                    sql.push_str(" AND event_ts_ms <= ?");
                    params_vec.push(Box::new(until));
                }
                sql.push_str(" ORDER BY seq");
                let mut stmt = conn.prepare(&sql)?;
                let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
                let mut rows = stmt.query(params_refs.as_slice())?;
                while let Some(r) = rows.next()? {
                    out.push(row_to_event(r)?);
                }
            }
            out.sort_by_key(|e| e.seq.0);
            out.dedup_by_key(|e| e.seq.0);
            let has_more = out.len() > limit;
            out.truncate(limit);
            let next_cursor = if has_more { out.last().map(|e| e.seq.0) } else { None };
            Ok(EventPage {
                events: out,
                next_cursor,
            })
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn health(&self) -> Result<Health, StoreError> {
        let pool = self.inner.read_pool.clone();
        let integrity_ok = self.inner.integrity_ok.load(Ordering::SeqCst);
        tokio::task::spawn_blocking(move || -> Result<Health, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
            let mut stmt =
                conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut tables = Vec::new();
            for r in rows {
                tables.push(r?);
            }
            let last_seq: i64 =
                conn.query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
            let migrations: i64 =
                conn.query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0))?;
            let wal_size = 0u64;
            Ok(Health {
                journal_mode,
                tables,
                migrations,
                wal_size_bytes: wal_size,
                last_seq,
                integrity_ok,
            })
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn overview(&self) -> Result<Overview, StoreError> {
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Overview, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let agents: i64 = conn.query_row("SELECT COUNT(*) FROM agents", [], |r| r.get(0))?;
            let alerts_open: i64 = conn.query_row(
                "SELECT COUNT(*) FROM alerts WHERE state IN ('open','acknowledged','snoozed')",
                [],
                |r| r.get(0),
            )?;
            let (req, inp): (i64, i64) = conn
                .query_row(
                    "SELECT COALESCE(SUM(requests),0), COALESCE(SUM(input_tokens),0) FROM usage_minute",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap_or((0, 0));
            Ok(Overview {
                agents,
                alerts_open,
                usage_requests: req,
                usage_input_tokens: inp,
            })
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn system_health(&self) -> Result<SystemHealthReport, StoreError> {
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<SystemHealthReport, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut stmt = conn.prepare(
                "SELECT ts_ms,cpu_percent,mem_percent,disk_percent,uptime_secs,docker_json,git_json,updated_seq FROM system_health ORDER BY ts_ms",
            )?;
            let rows = stmt.query_map([], projections::system_health::sample_from_row)?;
            let history = rows.collect::<Result<Vec<_>, _>>()?;
            Ok(SystemHealthReport { latest: history.last().cloned(), history })
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn rebuild(&self, opts: RebuildOptions) -> Result<RebuildReport, StoreError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.inner
            .writer_tx
            .send(WriteCmd::Rebuild {
                opts,
                db_path: self.inner.opts.path.clone(),
                reply: reply_tx,
            })
            .map_err(|_| StoreError::ChannelClosed)?;
        reply_rx.await.map_err(|_| StoreError::ChannelClosed)?
    }

    pub async fn backup(&self, dest: &Path) -> Result<(), StoreError> {
        let opts = self.inner.opts.clone();
        let dest = dest.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<(), StoreError> {
            if let Some(p) = dest.parent() {
                std::fs::create_dir_all(p)?;
            }
            let conn = Connection::open(&opts.path)?;
            conn.execute_batch(&format!("PRAGMA busy_timeout={};", opts.busy_timeout_ms))?;
            let tmp = dest.with_file_name(format!(
                "{}.tmp",
                dest.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "hq.db".to_string())
            ));
            let _ = std::fs::remove_file(&tmp);
            let mut dst = Connection::open(&tmp)?;
            let backup = rusqlite::backup::Backup::new(&conn, &mut dst)?;
            backup.run_to_completion(100, std::time::Duration::from_millis(10), None)?;
            // fsync dest file
            let f = std::fs::File::open(&tmp)?;
            f.sync_all()?;
            std::fs::rename(&tmp, &dest)?;
            // fsync dir
            if let Some(parent) = dest.parent()
                && let Ok(d) = std::fs::File::open(parent)
            {
                let _ = d.sync_all();
            }
            Ok(())
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }
}

fn row_to_event(r: &rusqlite::Row) -> Result<Event, StoreError> {
    let seq: i64 = r.get(0)?;
    let id: String = r.get(1)?;
    let event_ts_ms: i64 = r.get(2)?;
    let ingest_ts_ms: i64 = r.get(3)?;
    let kind_str: String = r.get(4)?;
    let subject_str: String = r.get(5)?;
    let source: String = r.get(6)?;
    let _schema: i64 = r.get(7)?;
    let corr: Option<String> = r.get(8)?;
    let caus: Option<String> = r.get(9)?;
    let idem: Option<String> = r.get(10)?;
    let payload_str: String = r.get(11)?;
    let kind = parse_kind(&kind_str);
    let subject = parse_subject(&subject_str, kind);
    let payload: serde_json::Value =
        serde_json::from_str(&payload_str).unwrap_or(serde_json::json!({}));
    let draft = EventDraft {
        event_ts_ms,
        kind,
        subject,
        source: SourceId(source),
        correlation_id: corr
            .filter(|s| !s.is_empty())
            .map(|s| EventId(ulid::Ulid::from_string(&s).unwrap_or_default())),
        causation_id: caus
            .filter(|s| !s.is_empty())
            .map(|s| EventId(ulid::Ulid::from_string(&s).unwrap_or_default())),
        idempotency_key: idem,
        payload,
    };
    Ok(Event {
        seq: Seq(seq),
        id: EventId(ulid::Ulid::from_string(&id).unwrap_or_default()),
        ingest_ts_ms,
        draft,
    })
}

fn parse_kind(s: &str) -> EventKind {
    for k in EventKind::ALL {
        if k.as_str() == s {
            return *k;
        }
    }
    EventKind::SystemHealthObserved
}

fn parse_subject(s: &str, kind: EventKind) -> Subject {
    use EventKind::*;
    if let Some(session_id) = s.strip_prefix("usage:session:") {
        if let Ok(session) = SessionId::parse(session_id) {
            return Subject::Usage(UsageScope::Session(session));
        }
        return Subject::Usage(UsageScope::Global);
    }
    if let Some(agent_id) = s.strip_prefix("usage:agent:") {
        return Subject::Usage(UsageScope::Agent(AgentId(format!("agent:{agent_id}"))));
    }
    if s == "usage:global" || s.starts_with("usage:") {
        return Subject::Usage(UsageScope::Global);
    }
    match kind {
        SessionStarted | SessionUpdated | SessionEnded => {
            return Subject::Session(SessionId(s.to_string()));
        }
        LogRecorded if SessionId::parse(s).is_ok() => {
            return Subject::Session(SessionId(s.to_string()));
        }
        AlertRaised | AlertUpdated | AlertAcknowledged | AlertSnoozed | AlertResolved => {
            return Subject::Alert(AlertId(s.to_string()));
        }
        ConnectorRegistered
        | ConnectorHealthChanged
        | ConnectorPollCompleted
        | ConnectorAuthFailed
        | ConnectorEvent => return Subject::Connector(ConnectorId(s.to_string())),
        ActionRequested | ActionApproved | ActionDenied | ActionExecuted | ActionFailed => {
            return Subject::Action(ActionId(s.to_string()));
        }
        DeviceAdded | DeviceRevoked => return Subject::Device(DeviceId(s.to_string())),
        KnowledgePageSynced | KnowledgePageQuarantined | KnowledgePromoted => {
            return Subject::Knowledge(s.strip_prefix("knowledge:").unwrap_or(s).to_string());
        }
        _ => {}
    }
    if s.starts_with("agent:") {
        return Subject::Agent(AgentId(s.to_string()));
    }
    if s == "system" {
        return Subject::System;
    }
    Subject::System
}

// writer loop
fn writer_loop(
    mut conn: Connection,
    rx: std::sync::mpsc::Receiver<WriteCmd>,
    _opts: StoreOptions,
    bc: broadcast::Sender<Patch>,
    projectors: Vec<Box<dyn projections::Projector>>,
) {
    while let Ok(cmd) = rx.recv() {
        match cmd {
            WriteCmd::Append {
                drafts,
                #[cfg(any(test, debug_assertions))]
                fail_after_insert,
                reply,
            } => {
                #[cfg(any(test, debug_assertions))]
                let fail = fail_after_insert;
                #[cfg(any(test, debug_assertions))]
                let res = do_append(&mut conn, drafts, fail, bc.clone(), &projectors);
                #[cfg(not(any(test, debug_assertions)))]
                let res = do_append(&mut conn, drafts, bc.clone(), &projectors);
                let _ = reply.send(res);
            }
            WriteCmd::Rebuild {
                opts,
                db_path,
                reply,
            } => {
                let res = do_rebuild(&mut conn, &projectors, opts, db_path);
                let _ = reply.send(res);
            }
            WriteCmd::AuthWrite { op, reply } => {
                let res = crate::auth::apply_write(&mut conn, op);
                let _ = reply.send(res);
            }
        }
    }
}

fn do_append(
    conn: &mut Connection,
    drafts: Vec<EventDraft>,
    #[cfg(any(test, debug_assertions))] fail_after_insert: bool,
    bc: broadcast::Sender<Patch>,
    projectors: &[Box<dyn projections::Projector>],
) -> Result<Vec<Event>, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut out_events = Vec::new();
    let mut patches: Vec<Patch> = Vec::new();
    // We need mutable tx; wrap in scope. Use a manual transaction object.
    // rusqlite transaction borrow; restructure: use conn.transaction then commit/rollback.
    // Already have tx; but need pass to projectors. We'll operate on tx.
    for mut d in drafts {
        hq_core::redact_json(&mut d.payload);
        d.validate().map_err(|e| StoreError::Core(e.to_string()))?;
        // idempotency
        if let Some(key) = &d.idempotency_key {
            let exists: Option<i64> = tx
                .query_row(
                    "SELECT seq FROM events WHERE source=?1 AND idempotency_key=?2",
                    params![d.source.0, key],
                    |r| r.get(0),
                )
                .ok();
            if exists.is_some() {
                continue;
            }
        }
        let id = EventId(ulid::Ulid::new());
        let ingest_ts_ms = d.event_ts_ms;
        tx.execute(
            "INSERT INTO events (id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,correlation_id,causation_id,idempotency_key,payload) VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8,?9,?10)",
            params![
                id.0.to_string(), d.event_ts_ms, ingest_ts_ms, d.kind.as_str(), d.subject.key(), d.source.0,
                d.correlation_id.map(|x| x.0.to_string()), d.causation_id.map(|x| x.0.to_string()),
                d.idempotency_key, serde_json::to_string(&d.payload).unwrap_or_default()
            ],
        )?;
        let seq = tx.last_insert_rowid();
        // failpoint (test-only)
        #[cfg(any(test, debug_assertions))]
        if fail_after_insert {
            tx.rollback()?;
            return Err(StoreError::Validation("failpoint".into()));
        }
        let ev = Event {
            seq: Seq(seq),
            id,
            ingest_ts_ms,
            draft: d.clone(),
        };
        for p in projectors {
            p.apply(&tx, &ev)?;
        }
        // patch
        let mut patch = patch_for_event(&ev);
        patch.seq = Seq(seq);
        patches.push(patch);
        out_events.push(ev);
    }
    tx.commit()?;
    for p in &patches {
        let _ = bc.send(p.clone());
    }
    Ok(out_events)
}

fn do_rebuild(
    conn: &mut Connection,
    projectors: &[Box<dyn projections::Projector>],
    opts: RebuildOptions,
    db_path: PathBuf,
) -> Result<RebuildReport, StoreError> {
    let start = std::time::Instant::now();
    // read all events ordered
    let mut events = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,correlation_id,causation_id,idempotency_key,payload FROM events ORDER BY seq")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            events.push(row_to_event(r)?);
        }
    }
    {
        let mut stmt = conn.prepare("SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,correlation_id,causation_id,idempotency_key,payload FROM events_archive ORDER BY seq")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            events.push(row_to_event(r)?);
        }
    }
    events.sort_by_key(|e| e.seq.0);
    events.dedup_by_key(|e| e.seq.0);
    let n = events.len();

    match opts {
        RebuildOptions::Shadow(path) => {
            if let Some(p) = path.parent() {
                std::fs::create_dir_all(p)?;
            }
            let _ = std::fs::remove_file(&path);
            // build shadow db
            let mut shconn = Connection::open(&path)?;
            shconn.execute_batch("PRAGMA journal_mode=WAL;")?;
            run_migrations(&mut shconn)?;
            {
                let tx = shconn.transaction_with_behavior(TransactionBehavior::Immediate)?;
                for p in projectors {
                    p.reset(&tx)?;
                }
                for ev in &events {
                    for p in projectors {
                        p.apply(&tx, ev)?;
                    }
                }
                let last_seq = events.last().map(|e| e.seq.0).unwrap_or(0);
                tx.execute(
                    "INSERT INTO projection_checkpoint (name,seq) VALUES ('all',?1) ON CONFLICT(name) DO UPDATE SET seq=excluded.seq",
                    params![last_seq],
                )?;
                tx.commit()?;
            }
            drop(shconn);
            // swap files
            let mut old_name = db_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "hq.db".to_string());
            old_name.push_str(".old");
            let old_path = db_path.with_file_name(old_name);
            let _ = std::fs::remove_file(&old_path);
            std::fs::rename(&db_path, &old_path)?;
            std::fs::rename(&path, &db_path)?;
            let _ = std::fs::remove_file(&old_path);
            if let Some(parent) = db_path.parent()
                && let Ok(d) = std::fs::File::open(parent)
            {
                let _ = d.sync_all();
            }
        }
        RebuildOptions::InPlaceTest => {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            for p in projectors {
                p.reset(&tx)?;
            }
            for ev in &events {
                for p in projectors {
                    p.apply(&tx, ev)?;
                }
            }
            let last_seq = events.last().map(|e| e.seq.0).unwrap_or(0);
            tx.execute(
                "INSERT INTO projection_checkpoint (name,seq) VALUES ('all',?1) ON CONFLICT(name) DO UPDATE SET seq=excluded.seq",
                params![last_seq],
            )?;
            tx.commit()?;
        }
    }

    Ok(RebuildReport {
        events: n,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
