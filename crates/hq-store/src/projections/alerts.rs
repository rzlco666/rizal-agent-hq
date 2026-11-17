use crate::error::StoreError;
use hq_core::*;
use rusqlite::Transaction;

pub fn apply(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    let id = ev
        .draft
        .payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("alert:1")
        .to_string();
    let fp = ev
        .draft
        .payload
        .get("fingerprint")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    let rule_id = ev
        .draft
        .payload
        .get("rule_id")
        .and_then(|v| v.as_str())
        .unwrap_or("rule")
        .to_string();
    let level = ev
        .draft
        .payload
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("info")
        .to_string();
    let title = ev
        .draft
        .payload
        .get("title")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());
    let actor = ev
        .draft
        .payload
        .get("actor")
        .and_then(|v| v.as_str())
        .unwrap_or("system")
        .to_string();
    let note = ev
        .draft
        .payload
        .get("note")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    match ev.draft.kind {
        EventKind::AlertRaised => {
            tx.execute(
                "INSERT OR IGNORE INTO alerts (id,fingerprint,rule_id,level,state,subject,title_json,detail_json,opened_ms,updated_ms,occurrences) VALUES (?1,?2,?3,?4,'open',?5,?6,'{}',?7,?7,1)",
                rusqlite::params![id, fp, rule_id, level, ev.draft.subject.key(), title, ev.draft.event_ts_ms],
            )?;
        }
        EventKind::AlertUpdated => {
            tx.execute(
                "UPDATE alerts SET level=?2, updated_ms=?3 WHERE id=?1",
                rusqlite::params![id, level, ev.draft.event_ts_ms],
            )?;
        }
        EventKind::AlertAcknowledged => {
            tx.execute(
                "UPDATE alerts SET state='acknowledged', ack_ms=?2, updated_ms=?2 WHERE id=?1",
                rusqlite::params![id, ev.draft.event_ts_ms],
            )?;
            tx.execute(
                "INSERT INTO alert_decisions (alert_id,actor,action,at_ms,note) VALUES (?1,?2,'acknowledge',?3,?4)",
                rusqlite::params![id, actor, ev.draft.event_ts_ms, note],
            )?;
        }
        EventKind::AlertSnoozed => {
            let until = ev
                .draft
                .payload
                .get("snooze_until_ms")
                .and_then(|v| v.as_i64())
                .unwrap_or(ev.draft.event_ts_ms);
            tx.execute(
                "UPDATE alerts SET state='snoozed', snooze_until_ms=?2, updated_ms=?3 WHERE id=?1",
                rusqlite::params![id, until, ev.draft.event_ts_ms],
            )?;
            tx.execute(
                "INSERT INTO alert_decisions (alert_id,actor,action,at_ms,note) VALUES (?1,?2,'snooze',?3,?4)",
                rusqlite::params![id, actor, ev.draft.event_ts_ms, note],
            )?;
        }
        EventKind::AlertResolved => {
            tx.execute(
                "UPDATE alerts SET state='resolved', resolved_ms=?2, updated_ms=?2 WHERE id=?1",
                rusqlite::params![id, ev.draft.event_ts_ms],
            )?;
            tx.execute(
                "INSERT INTO alert_decisions (alert_id,actor,action,at_ms,note) VALUES (?1,?2,'resolve',?3,?4)",
                rusqlite::params![id, actor, ev.draft.event_ts_ms, note],
            )?;
        }
        _ => return Ok(()),
    }
    Ok(())
}

pub struct AlertProjector;
impl crate::projections::Projector for AlertProjector {
    fn name(&self) -> &'static str {
        "alerts"
    }
    fn reset(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), crate::error::StoreError> {
        tx.execute("DELETE FROM alerts", [])?;
        tx.execute("DELETE FROM alert_decisions", [])?;
        Ok(())
    }
    fn apply(
        &self,
        tx: &rusqlite::Transaction<'_>,
        ev: &Event,
    ) -> Result<(), crate::error::StoreError> {
        apply(tx, ev)
    }
}
