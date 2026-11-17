use crate::error::StoreError;
use hq_core::*;
use rusqlite::Transaction;

pub fn apply(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    if ev.draft.kind != EventKind::UsageRecorded {
        return Ok(());
    }
    let agent = ev.draft.subject.key();
    // usage subject may be usage:agent:...; if agent subject, use key
    let agent_id = if let Subject::Agent(a) = &ev.draft.subject {
        a.0.clone()
    } else {
        agent.clone()
    };
    let tokens: i64 = ev
        .draft
        .payload
        .get("tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let bucket = ev.draft.event_ts_ms - (ev.draft.event_ts_ms % 60000);
    let day = "1970-01-01".to_string();
    tx.execute(
        "INSERT INTO usage_minute (bucket_ms,agent_id,provider,model,attribution,requests,input_tokens,output_tokens,cache_tokens,cash_usd,shadow_usd) VALUES (?1,?2,'p','m','exact',1,?3,0,0,0,0) ON CONFLICT(bucket_ms,agent_id,provider,model,attribution) DO UPDATE SET requests=requests+1,input_tokens=input_tokens+excluded.input_tokens",
        rusqlite::params![bucket, agent_id, tokens],
    )?;
    tx.execute(
        "INSERT INTO usage_daily (day,agent_id,provider,model,attribution,requests,input_tokens,output_tokens,cache_tokens,cash_usd,shadow_usd) VALUES (?1,?2,'p','m','exact',1,?3,0,0,0,0) ON CONFLICT(day,agent_id,provider,model,attribution) DO UPDATE SET requests=requests+1,input_tokens=input_tokens+excluded.input_tokens",
        rusqlite::params![day, agent_id, tokens],
    )?;
    Ok(())
}
