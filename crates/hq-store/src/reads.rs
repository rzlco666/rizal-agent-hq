use crate::{Locale, Sentence, Store, StoreError};
use hq_humanize::render;
use rusqlite::{OptionalExtension, params};

#[derive(Debug, Clone)]
pub struct TaskView {
    pub id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub title: String,
    pub phase: String,
    pub started_ms: i64,
    pub updated_ms: i64,
    pub ended_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SessionView {
    pub id: String,
    pub agent_id: String,
    pub runtime: String,
    pub started_ms: i64,
    pub ended_ms: Option<i64>,
    pub outcome: Option<String>,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cash_usd: f64,
}

#[derive(Debug, Clone)]
pub struct AlertView {
    pub id: String,
    pub fingerprint: String,
    pub rule_id: String,
    pub level: String,
    pub state: String,
    pub subject: String,
    pub title: String,
    pub opened_ms: i64,
    pub updated_ms: i64,
    pub ack_ms: Option<i64>,
    pub snooze_until_ms: Option<i64>,
    pub resolved_ms: Option<i64>,
    pub occurrences: i64,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub agent_id: String,
    pub changed_ms: i64,
}

#[derive(Debug, Clone, Default)]
pub struct CostSummary {
    pub cash_usd: f64,
    pub shadow_usd: f64,
}

fn row_to_task(row: &rusqlite::Row<'_>, locale: Locale) -> Result<TaskView, rusqlite::Error> {
    let title_json: String = row.get(3)?;
    let sentence: Sentence = serde_json::from_str(&title_json).unwrap_or_else(|_| Sentence {
        key: "task".to_string(),
        params: Default::default(),
    });
    Ok(TaskView {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        session_id: row.get(2)?,
        title: render(&sentence, locale),
        phase: row.get(4)?,
        started_ms: row.get(5)?,
        updated_ms: row.get(6)?,
        ended_ms: row.get(7)?,
    })
}
fn row_to_session(row: &rusqlite::Row<'_>) -> Result<SessionView, rusqlite::Error> {
    Ok(SessionView {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        runtime: row.get(2)?,
        started_ms: row.get(3)?,
        ended_ms: row.get(4)?,
        outcome: row.get(5)?,
        requests: row.get(6)?,
        input_tokens: row.get(7)?,
        output_tokens: row.get(8)?,
        cash_usd: row.get(9)?,
    })
}

fn row_to_alert(row: &rusqlite::Row<'_>) -> Result<AlertView, rusqlite::Error> {
    let title_json: String = row.get(6)?;
    let title: serde_json::Value =
        serde_json::from_str(&title_json).unwrap_or(serde_json::Value::Null);
    Ok(AlertView {
        id: row.get(0)?,
        fingerprint: row.get(1)?,
        rule_id: row.get(2)?,
        level: row.get(3)?,
        state: row.get(4)?,
        subject: row.get(5)?,
        title: title
            .get("key")
            .and_then(|k| k.as_str())
            .unwrap_or("alert.raised")
            .to_string(),
        opened_ms: row.get(8)?,
        updated_ms: row.get(9)?,
        ack_ms: row.get(10)?,
        snooze_until_ms: row.get(11)?,
        resolved_ms: row.get(12)?,
        occurrences: row.get(13)?,
    })
}

impl Store {
    pub async fn tasks(
        &self,
        agent: Option<&str>,
        phase: Option<&str>,
        q: Option<&str>,
        locale: Locale,
        limit: usize,
    ) -> Result<Vec<TaskView>, StoreError> {
        let pool = self.inner.read_pool.clone();
        let agent = agent.map(str::to_string);
        let phase = phase.map(str::to_string);
        let q = q.map(|q| format!("%{q}%"));
        tokio::task::spawn_blocking(move || -> Result<Vec<TaskView>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut sql = "SELECT id,agent_id,session_id,title_json,phase,started_ms,updated_ms,ended_ms FROM tasks WHERE 1=1".to_string();
            let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(agent) = agent {
                sql.push_str(" AND agent_id=?");
                values.push(Box::new(agent));
            }
            if let Some(phase) = phase {
                sql.push_str(" AND phase=?");
                values.push(Box::new(phase));
            }
            if let Some(q) = q {
                sql.push_str(" AND title_json LIKE ?");
                values.push(Box::new(q));
            }
            sql.push_str(" ORDER BY updated_ms DESC,id LIMIT ?");
            values.push(Box::new(limit.min(500) as i64));
            let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(refs.as_slice(), |row| row_to_task(row, locale))?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn task(&self, id: &str, locale: Locale) -> Result<Option<TaskView>, StoreError> {
        let pool = self.inner.read_pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<TaskView>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            Ok(conn
                .query_row(
                    "SELECT id,agent_id,session_id,title_json,phase,started_ms,updated_ms,ended_ms FROM tasks WHERE id=?1",
                    params![id],
                    |row| row_to_task(row, locale),
                )
                .optional()?)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn sessions(
        &self,
        agent: &str,
        limit: usize,
    ) -> Result<Vec<SessionView>, StoreError> {
        let pool = self.inner.read_pool.clone();
        let agent = agent.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<SessionView>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut stmt = conn.prepare(
                "SELECT id,agent_id,runtime,started_ms,ended_ms,outcome,requests,input_tokens,output_tokens,cash_usd \
                 FROM sessions WHERE agent_id=?1 ORDER BY started_ms DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![agent, limit.min(500) as i64], row_to_session)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn files(&self, agent: &str) -> Result<Vec<ChangedFile>, StoreError> {
        // File facts are not projected in v1; an empty page is honest and typed.
        let _ = agent;
        Ok(Vec::new())
    }

    pub async fn alerts(
        &self,
        state: Option<&str>,
        level: Option<&str>,
    ) -> Result<Vec<AlertView>, StoreError> {
        let pool = self.inner.read_pool.clone();
        let state = state.map(str::to_string);
        let level = level.map(str::to_string);
        tokio::task::spawn_blocking(move || -> Result<Vec<AlertView>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut sql = "SELECT id,fingerprint,rule_id,level,state,subject,title_json,detail_json,opened_ms,updated_ms,ack_ms,snooze_until_ms,resolved_ms,occurrences FROM alerts WHERE 1=1".to_string();
            let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(state) = state {
                sql.push_str(" AND state=?");
                values.push(Box::new(state));
            }
            if let Some(level) = level {
                sql.push_str(" AND level=?");
                values.push(Box::new(level));
            }
            sql.push_str(" ORDER BY updated_ms DESC,id");
            let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(refs.as_slice(), row_to_alert)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn alert(&self, id: &str) -> Result<Option<AlertView>, StoreError> {
        let pool = self.inner.read_pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<AlertView>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            Ok(conn
                .query_row(
                    "SELECT id,fingerprint,rule_id,level,state,subject,title_json,detail_json,opened_ms,updated_ms,ack_ms,snooze_until_ms,resolved_ms,occurrences FROM alerts WHERE id=?1",
                    params![id],
                    row_to_alert,
                )
                .optional()?)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn costs(&self) -> Result<CostSummary, StoreError> {
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<CostSummary, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let (cash, shadow): (f64, f64) = conn.query_row(
                "SELECT COALESCE(SUM(cash_usd),0),COALESCE(SUM(shadow_usd),0) FROM usage_minute",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            Ok(CostSummary {
                cash_usd: cash,
                shadow_usd: shadow,
            })
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }
}
