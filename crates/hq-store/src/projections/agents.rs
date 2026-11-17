use crate::error::StoreError;
use hq_core::*;
use rusqlite::Transaction;

pub fn apply(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    match ev.draft.kind {
        EventKind::AgentObserved
        | EventKind::AgentSeen
        | EventKind::AgentStatusChanged
        | EventKind::AgentDisappeared => {}
        _ => return Ok(()),
    }
    let id = ev.draft.subject.key();
    let name = id.split(':').next_back().unwrap_or(&id).to_uppercase();
    let status = ev
        .draft
        .payload
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "bekerja" => Status::Bekerja,
            "berpikir" => Status::Berpikir,
            "menunggu-rizal" => Status::MenungguRizal,
            "santai" => Status::Santai,
            "offline" => Status::Offline,
            "error" => Status::Error,
            _ => Status::Bekerja,
        })
        .unwrap_or(Status::Bekerja);
    let last_seen = ev.draft.event_ts_ms;
    tx.execute(
        "INSERT INTO agents (id,name,role,kind,parent_id,runtime_key,status,activity_json,provider,model,evidence,last_seen_ms,updated_seq) VALUES (?1,?2,'agent','utama',NULL,?3,?4,'{}',NULL,NULL,'L1',?5,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,status=excluded.status,last_seen_ms=excluded.last_seen_ms,updated_seq=excluded.updated_seq",
        rusqlite::params![id, name, id, status_to_str(status), last_seen, ev.seq.0],
    )?;
    Ok(())
}

fn status_to_str(s: Status) -> &'static str {
    match s {
        Status::Bekerja => "bekerja",
        Status::Berpikir => "berpikir",
        Status::MenungguRizal => "menunggu-rizal",
        Status::Santai => "santai",
        Status::Offline => "offline",
        Status::Error => "error",
    }
}
