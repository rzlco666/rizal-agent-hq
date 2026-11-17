use crate::LogLevel;
use crate::error::StoreError;
use crate::{Projector, Sentence};
use hq_core::{Event, EventKind, Subject, redact_text};
use hq_humanize::render;
use rusqlite::{Transaction, params};
use serde_json::Value;
use std::collections::BTreeMap;

pub struct LogProjector;

impl Projector for LogProjector {
    fn name(&self) -> &'static str {
        "logs"
    }

    fn reset(&self, tx: &Transaction<'_>) -> Result<(), StoreError> {
        tx.execute("DELETE FROM logs_fts", [])?;
        tx.execute("DELETE FROM logs", [])?;
        Ok(())
    }

    fn apply(&self, tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
        if ev.draft.kind != EventKind::LogRecorded {
            return Ok(());
        }
        let (agent_id, session_id) = match &ev.draft.subject {
            Subject::Agent(agent) => (agent.0.clone(), None),
            Subject::Session(session) => (
                session
                    .0
                    .split_once("/session/")
                    .map_or(session.0.as_str(), |(agent, _)| agent)
                    .to_string(),
                Some(session.0.clone()),
            ),
            _ => return Ok(()),
        };

        let sentence = sentence_from_payload(&ev.draft.payload, &agent_id);
        let sentence_json =
            serde_json::to_string(&sentence).map_err(|e| StoreError::Core(e.to_string()))?;
        let level = LogLevel::parse(
            ev.draft
                .payload
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or("info"),
        );
        let raw = ev
            .draft
            .payload
            .get("raw")
            .and_then(Value::as_str)
            .map(redact_text);
        let id = format!("log:{}", ev.id.0);
        tx.execute(
            "INSERT INTO logs (id,agent_id,session_id,ts_ms,level,sentence_json,raw,seq)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(id) DO UPDATE SET ts_ms=excluded.ts_ms, level=excluded.level,
               sentence_json=excluded.sentence_json, raw=excluded.raw",
            params![
                id,
                agent_id,
                session_id,
                ev.draft.event_ts_ms,
                level.as_str(),
                sentence_json,
                raw,
                ev.seq.0
            ],
        )?;
        tx.execute("DELETE FROM logs_fts WHERE log_id=?1", params![id])?;
        tx.execute(
            "INSERT INTO logs_fts (log_id,text) VALUES (?1,?2)",
            params![id, render(&sentence, hq_humanize::Locale::Id)],
        )?;
        Ok(())
    }
}

fn sentence_from_payload(payload: &Value, agent: &str) -> Sentence {
    if let Ok(sentence) =
        serde_json::from_value::<Sentence>(payload.get("sentence").cloned().unwrap_or_default())
    {
        return sentence;
    }
    // Raw-only logs stay a template plus params; never finalize the text.
    let mut params = BTreeMap::new();
    params.insert("agent".to_string(), redact_text(agent));
    params.insert(
        "detail".to_string(),
        redact_text(payload.get("message").and_then(Value::as_str).unwrap_or("")),
    );
    Sentence {
        key: "agent.logged".to_string(),
        params,
    }
}
