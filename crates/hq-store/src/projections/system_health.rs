use crate::error::StoreError;
use hq_core::{Event, EventKind};
use rusqlite::{Transaction, params};

const HISTORY_MS: i64 = 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, PartialEq)]
pub struct SystemHealthSample {
    pub ts_ms: i64,
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub disk_percent: f64,
    pub uptime_secs: u64,
    pub docker: Vec<serde_json::Value>,
    pub git: Vec<serde_json::Value>,
    pub updated_seq: i64,
}

pub fn apply(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    if ev.draft.kind != EventKind::SystemHealthObserved {
        return Ok(());
    }
    let payload = &ev.draft.payload;
    let docker = payload
        .get("docker")
        .cloned()
        .unwrap_or(serde_json::Value::Null)
        .to_string();
    let git = payload
        .get("git")
        .cloned()
        .unwrap_or(serde_json::Value::Null)
        .to_string();
    tx.execute(
        "INSERT OR REPLACE INTO system_health (ts_ms,cpu_percent,mem_percent,disk_percent,uptime_secs,docker_json,git_json,updated_seq) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            ev.draft.event_ts_ms,
            payload.get("cpu_percent").and_then(serde_json::Value::as_f64).unwrap_or_default(),
            payload.get("mem_percent").and_then(serde_json::Value::as_f64).unwrap_or_default(),
            payload.get("disk_percent").and_then(serde_json::Value::as_f64).unwrap_or_default(),
            payload.get("uptime_secs").and_then(serde_json::Value::as_i64).unwrap_or_default(),
            docker,
            git,
            ev.seq.0,
        ],
    )?;
    tx.execute(
        "DELETE FROM system_health WHERE ts_ms < ?1",
        [ev.draft.event_ts_ms.saturating_sub(HISTORY_MS)],
    )?;
    Ok(())
}

pub struct SystemHealthProjector;
impl crate::projections::Projector for SystemHealthProjector {
    fn name(&self) -> &'static str {
        "system_health"
    }
    fn reset(&self, tx: &Transaction<'_>) -> Result<(), StoreError> {
        tx.execute("DELETE FROM system_health", [])?;
        Ok(())
    }
    fn apply(&self, tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
        apply(tx, ev)
    }
}

pub fn sample_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SystemHealthSample> {
    let docker_json: String = row.get(5)?;
    let git_json: String = row.get(6)?;
    Ok(SystemHealthSample {
        ts_ms: row.get(0)?,
        cpu_percent: row.get(1)?,
        mem_percent: row.get(2)?,
        disk_percent: row.get(3)?,
        uptime_secs: row.get(4)?,
        docker: serde_json::from_str(&docker_json).unwrap_or_default(),
        git: serde_json::from_str(&git_json).unwrap_or_default(),
        updated_seq: row.get(7)?,
    })
}
