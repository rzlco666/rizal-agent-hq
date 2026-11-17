use crate::TaskProjection;
use crate::error::StoreError;
use crate::{Projector, TaskDeriver};
use hq_core::{Event, EventKind, Subject};
use rusqlite::{Transaction, params};

pub struct TaskProjector;

impl Projector for TaskProjector {
    fn name(&self) -> &'static str {
        "tasks"
    }

    fn reset(&self, tx: &Transaction<'_>) -> Result<(), StoreError> {
        tx.execute("DELETE FROM task_steps", [])?;
        tx.execute("DELETE FROM tasks", [])?;
        Ok(())
    }

    fn apply(&self, tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
        let subject_key = match &ev.draft.subject {
            Subject::Session(session) => session.0.clone(),
            Subject::Agent(agent) if ev.draft.kind == EventKind::LogRecorded => agent.0.clone(),
            _ => return Ok(()),
        };
        if !matches!(
            ev.draft.kind,
            EventKind::SessionStarted
                | EventKind::SessionUpdated
                | EventKind::SessionEnded
                | EventKind::LogRecorded
        ) {
            return Ok(());
        }

        let events = load_events(tx, &subject_key)?;
        for task in TaskDeriver::default().derive(&events) {
            upsert_task(tx, &task, ev.seq.0)?;
        }
        Ok(())
    }
}

fn load_events(tx: &Transaction<'_>, subject: &str) -> Result<Vec<Event>, StoreError> {
    let mut stmt = tx.prepare(
        "SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,
                correlation_id,causation_id,idempotency_key,payload
         FROM (
           SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,
                  correlation_id,causation_id,idempotency_key,payload
           FROM events WHERE subject=?1
           UNION ALL
           SELECT seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,
                  correlation_id,causation_id,idempotency_key,payload
           FROM events_archive WHERE subject=?1
         ) ORDER BY seq",
    )?;
    let mut rows = stmt.query(params![subject])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(crate::row_to_event(row)?);
    }
    Ok(out)
}

fn upsert_task(tx: &Transaction<'_>, task: &TaskProjection, seq: i64) -> Result<(), StoreError> {
    let title = serde_json::to_string(&task.title).map_err(|e| StoreError::Core(e.to_string()))?;
    let phase = serde_json::to_value(task.phase)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let updated_ms = task.ended_ms.unwrap_or(task.started_ms);
    let session_id = (!task.session_id.is_empty()).then(|| task.session_id.clone());
    tx.execute(
        "INSERT INTO tasks (id,agent_id,session_id,title_json,phase,started_ms,updated_ms,
                            ended_ms,outcome_json,ordinal,updated_seq)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,NULL,?9,?10)
         ON CONFLICT(id) DO UPDATE SET title_json=excluded.title_json, phase=excluded.phase,
           started_ms=excluded.started_ms, updated_ms=excluded.updated_ms,
           ended_ms=excluded.ended_ms, updated_seq=excluded.updated_seq",
        params![
            task.id,
            task.agent_id,
            session_id,
            title,
            phase,
            task.started_ms,
            updated_ms,
            task.ended_ms,
            0,
            seq
        ],
    )?;

    tx.execute("DELETE FROM task_steps WHERE task_id=?1", params![task.id])?;
    for (idx, step) in task.steps.iter().enumerate() {
        let label =
            serde_json::to_string(&step.label).map_err(|e| StoreError::Core(e.to_string()))?;
        tx.execute(
            "INSERT INTO task_steps (task_id,idx,label_json,done,at_ms) VALUES (?1,?2,?3,0,?4)",
            params![task.id, idx as i64, label, step.at_ms],
        )?;
    }

    Ok(())
}
