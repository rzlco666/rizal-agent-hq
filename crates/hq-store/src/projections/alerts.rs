use crate::error::StoreError;
use hq_core::*;
use rusqlite::Transaction;

pub fn apply(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    match ev.draft.kind {
        EventKind::AlertRaised => {}
        _ => return Ok(()),
    }
    let id = ev
        .draft
        .payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("alert:1")
        .to_string();
    let fp = id.clone();
    tx.execute(
        "INSERT OR IGNORE INTO alerts (id,fingerprint,rule_id,level,state,subject,title_json,detail_json,opened_ms,updated_ms,occurrences) VALUES (?1,?2,'rule','info','open',?3,'{}','{}',?4,?4,1)",
        rusqlite::params![id, fp, ev.draft.subject.key(), ev.draft.event_ts_ms],
    )?;
    Ok(())
}
