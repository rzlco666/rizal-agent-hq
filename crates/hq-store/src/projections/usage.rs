use crate::error::StoreError;
use hq_core::*;
use rusqlite::Transaction;

pub fn apply(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    if ev.draft.kind != EventKind::UsageRecorded {
        return Ok(());
    }
    let agent_id = if let Subject::Agent(a) = &ev.draft.subject {
        a.0.clone()
    } else {
        ev.draft.subject.key()
    };
    let requests: i64 = ev
        .draft
        .payload
        .get("requests")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    let input_tokens: i64 = ev
        .draft
        .payload
        .get("tokens")
        .or_else(|| ev.draft.payload.get("input_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let output_tokens: i64 = ev
        .draft
        .payload
        .get("output_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let cache_tokens: i64 = ev
        .draft
        .payload
        .get("cache_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let cash_usd: f64 = ev
        .draft
        .payload
        .get("cash_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let shadow_usd: f64 = ev
        .draft
        .payload
        .get("shadow_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let bucket = ev.draft.event_ts_ms - (ev.draft.event_ts_ms % 60000);
    // UTC day YYYY-MM-DD
    let secs = ev.draft.event_ts_ms / 1000;
    let days = secs.div_euclid(86400);
    // civil date from days since epoch
    let (y, m, d) = days_to_ymd(days);
    let day = format!("{:04}-{:02}-{:02}", y, m, d);

    tx.execute(
        "INSERT INTO usage_minute (bucket_ms,agent_id,provider,model,attribution,requests,input_tokens,output_tokens,cache_tokens,cash_usd,shadow_usd) VALUES (?1,?2,'p','m','exact',?3,?4,?5,?6,?7,?8) ON CONFLICT(bucket_ms,agent_id,provider,model,attribution) DO UPDATE SET requests=requests+excluded.requests,input_tokens=input_tokens+excluded.input_tokens,output_tokens=output_tokens+excluded.output_tokens,cache_tokens=cache_tokens+excluded.cache_tokens,cash_usd=cash_usd+excluded.cash_usd,shadow_usd=shadow_usd+excluded.shadow_usd",
        rusqlite::params![bucket, agent_id, requests, input_tokens, output_tokens, cache_tokens, cash_usd, shadow_usd],
    )?;
    tx.execute(
        "INSERT INTO usage_daily (day,agent_id,provider,model,attribution,requests,input_tokens,output_tokens,cache_tokens,cash_usd,shadow_usd) VALUES (?1,?2,'p','m','exact',?3,?4,?5,?6,?7,?8) ON CONFLICT(day,agent_id,provider,model,attribution) DO UPDATE SET requests=requests+excluded.requests,input_tokens=input_tokens+excluded.input_tokens,output_tokens=output_tokens+excluded.output_tokens,cache_tokens=cache_tokens+excluded.cache_tokens,cash_usd=cash_usd+excluded.cash_usd,shadow_usd=shadow_usd+excluded.shadow_usd",
        rusqlite::params![day, agent_id, requests, input_tokens, output_tokens, cache_tokens, cash_usd, shadow_usd],
    )?;
    Ok(())
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Howard Hinnant civil_from_days
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

pub struct UsageProjector;
impl crate::projections::Projector for UsageProjector {
    fn name(&self) -> &'static str {
        "usage"
    }
    fn reset(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), crate::error::StoreError> {
        tx.execute("DELETE FROM usage_minute", [])?;
        tx.execute("DELETE FROM usage_daily", [])?;
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
