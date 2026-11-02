//! Pembaca sesi OMP — sumber humanisasi tingkat sesi (M2).
//! Dua sumber digabung:
//!  1. File JSONL sesi di ~/.omp/agent/sessions/<folder>/*.jsonl
//!     (prompt pertama, model, panggilan tool terakhir + maksud "i"-nya).
//!  2. ~/.omp/stats.db tabel session_rollup/messages/tool_calls
//!     (request, token, biaya, dan tipe agent main/subagent PER SESI — asli, bukan tebakan).

use crate::omp;
use hq_core::redact::redact;
use hq_core::Usage;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ToolUse {
    pub name: String,
    pub intent: String,
    pub target: String,
}

#[derive(Debug, Clone, Default)]
pub struct SessionInfo {
    pub file: String,
    pub id: String,
    /// Nama tugas dari nama berkas sesi subagent (mis. "IsolationSuite"); kosong untuk sesi utama.
    pub name: String,
    /// True bila sesi ini anak dari sesi lain (parentSession di header / agent_type subagent).
    pub is_subagent: bool,
    pub project: String,
    pub cwd: String,
    pub started_ms: i64,
    pub last_ms: i64,
    pub ended: bool,
    pub model: String,
    /// "main" | "subagent" | "" (dari tool_calls.agent_type dominan)
    pub agent_type: String,
    pub first_prompt: String,
    pub last_tools: Vec<ToolUse>,
    pub tool_counts: HashMap<String, u32>,
    pub errors: u32,
    pub usage: Usage,
}

/// Parse "2026-11-02T08:39:03.805Z" -> epoch ms (UTC).
pub fn iso_to_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 {
        return None;
    }
    let num = |a: usize, z: usize| -> Option<i64> { s.get(a..z)?.parse().ok() };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, sec) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let ms: i64 = if b.get(19) == Some(&b'.') { num(20, 23).unwrap_or(0) } else { 0 };
    // days_from_civil (Howard Hinnant)
    let yy = if mo <= 2 { y - 1 } else { y };
    let era = yy.div_euclid(400);
    let yoe = yy.rem_euclid(400);
    let mp = if mo > 2 { mo - 3 } else { mo + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(((days * 86400 + h * 3600 + mi * 60 + sec) * 1000) + ms)
}

fn clean_prompt(text: &str) -> String {
    for line in text.lines() {
        let l = line.trim().trim_start_matches('#').trim();
        if l.len() >= 12 {
            return redact(l).chars().take(140).collect();
        }
    }
    redact(text.trim()).chars().take(140).collect()
}

pub fn parse_session_file(path: &std::path::Path) -> Option<SessionInfo> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut s = SessionInfo {
        file: path.to_string_lossy().to_string(),
        ..Default::default()
    };
    let fname = path.file_stem()?.to_string_lossy().to_string();
    if let Some((_, uuid)) = fname.split_once('_') {
        s.id = uuid.to_string();
    }
    // Berkas sesi subagent dinamai menurut tugasnya (bukan tanggal_UUID).
    if !fname.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        s.name = fname.clone();
    }
    for line in text.lines() {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        match v.get("type").and_then(|t| t.as_str()) {
            Some("session") => {
                s.id = v.get("id").and_then(|x| x.as_str()).unwrap_or(&s.id).to_string();
                s.cwd = v.get("cwd").and_then(|x| x.as_str()).unwrap_or("").to_string();
                s.started_ms = v.get("timestamp").and_then(|x| x.as_str()).and_then(iso_to_ms).unwrap_or(0);
                if v.get("parentSession").is_some() {
                    s.is_subagent = true;
                }
            }
            Some("model_change") => {
                if let Some(m) = v.get("model").and_then(|x| x.as_str()) {
                    s.model = m.to_string();
                }
            }
            Some("message") => {
                if let Some(ts) = v.get("timestamp").and_then(|x| x.as_str()).and_then(iso_to_ms) {
                    s.last_ms = s.last_ms.max(ts);
                }
                let m = v.get("message")?;
                let role = m.get("role").and_then(|x| x.as_str()).unwrap_or("");
                let content = m.get("content").and_then(|x| x.as_array());
                if role == "user" && s.first_prompt.is_empty() {
                    if let Some(parts) = content {
                        for p in parts {
                            if p.get("type").and_then(|x| x.as_str()) == Some("text") {
                                if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
                                    s.first_prompt = clean_prompt(t);
                                    break;
                                }
                            }
                        }
                    }
                }
                if role == "assistant" {
                    if let Some(parts) = content {
                        for p in parts {
                            if p.get("type").and_then(|x| x.as_str()) == Some("toolCall") {
                                let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let args = p.get("arguments").cloned().unwrap_or(Value::Null);
                                let intent = args.get("i").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let mut target = args
                                    .get("path")
                                    .or_else(|| args.get("file"))
                                    .or_else(|| args.get("command"))
                                    .or_else(|| args.get("pattern"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                // Perkakas edit: sasarannya di baris pertama input "[<path>#...]".
                                if target.is_empty() && name == "edit" {
                                    if let Some(inp) = args.get("input").and_then(|x| x.as_str()) {
                                        if let Some(first) = inp.lines().next() {
                                            let f = first.trim().trim_start_matches('[');
                                            target = f.split('#').next().unwrap_or("").to_string();
                                        }
                                    }
                                }
                                *s.tool_counts.entry(name.clone()).or_default() += 1;
                                s.last_tools.push(ToolUse { name, intent, target: redact(&target).chars().take(110).collect() });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if s.last_tools.len() > 8 {
        let keep = s.last_tools.split_off(s.last_tools.len() - 8);
        s.last_tools = keep;
    }
    s.project = s
        .cwd
        .rsplit('/')
        .next()
        .filter(|x| !x.is_empty())
        .unwrap_or("project")
        .to_string();
    if s.started_ms == 0 {
        return None;
    }
    Some(s)
}

/// Baca stats.db (read-only): lengkapi usage & tipe agent per sesi.
fn enrich_from_stats_db(sessions: &mut Vec<SessionInfo>) {
    let db_path = format!("{}/.omp/stats.db", crate::home());
    let conn = match rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(_) => return,
    };
    for s in sessions.iter_mut() {
        // session_rollup: request/token/biaya per file sesi.
        let row = conn.query_row(
            "SELECT requests, total_tokens, cost_total, ended_at FROM session_rollup WHERE session_file = ?1",
            rusqlite::params![s.file],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                ))
            },
        );
        if let Ok((req, _tokens, cost, ended_at)) = row {
            s.usage.requests = req.max(0) as u64;
            s.ended = ended_at.map(|e| e > 0).unwrap_or(false);
        }
        // Token per provider dari messages -> pisah biaya proporsional per kelas provider.
        let mut stmt = match conn.prepare(
            "SELECT provider, COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COALESCE(SUM(cache_read_tokens),0), COUNT(*) FROM messages WHERE session_file = ?1 GROUP BY provider",
        ) {
            Ok(st) => st,
            Err(_) => continue,
        };
        let rows: Vec<(String, i64, i64, i64)> = stmt
            .query_map(rusqlite::params![s.file], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
            })
            .map(|it| it.flatten().collect())
            .unwrap_or_default();
        // Bila session_rollup belum punya baris sesi ini, jumlah request dari messages.
        if s.usage.requests == 0 {
            if let Ok(cnt) = conn.query_row(
                "SELECT COUNT(*) FROM messages WHERE session_file = ?1",
                rusqlite::params![s.file],
                |r| r.get::<_, i64>(0),
            ) {
                s.usage.requests = cnt.max(0) as u64;
            }
        }
        let total_tok: i64 = rows.iter().map(|(_, i, o, _)| i + o).sum();
        let cost_total = if let Ok((_, _, c, _)) = conn.query_row(
            "SELECT requests, total_tokens, cost_total, ended_at FROM session_rollup WHERE session_file = ?1",
            rusqlite::params![s.file],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?, r.get::<_, Option<i64>>(3)?)),
        ) {
            c
        } else {
            0.0
        };
        for (provider, inp, outp, cache) in &rows {
            s.usage.input_tokens += (*inp).max(0) as u64;
            s.usage.output_tokens += (*outp).max(0) as u64;
            s.usage.cache_tokens += (*cache).max(0) as u64;
            if total_tok > 0 && cost_total > 0.0 {
                let share = (*inp + *outp) as f64 / total_tok as f64;
                let (cash, shadow) = omp::split_one(provider, &s.model, cost_total * share);
                s.usage.cash_usd += cash;
                s.usage.shadow_usd += shadow;
            }
        }
        // Tipe agent dominan dari tool_calls.
        if let Ok(at) = conn.query_row(
            "SELECT agent_type FROM tool_calls WHERE session_file = ?1 GROUP BY agent_type ORDER BY COUNT(*) DESC LIMIT 1",
            rusqlite::params![s.file],
            |r| r.get::<_, String>(0),
        ) {
            s.agent_type = at;
        }
        // Error dari messages.
        if let Ok(e) = conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE session_file = ?1 AND error_message IS NOT NULL AND error_message != ''",
            rusqlite::params![s.file],
            |r| r.get::<_, i64>(0),
        ) {
            s.errors = e.max(0) as u32;
        }
    }
}

/// Kumpulkan sesi dari semua folder project, terbaru dulu. max_age dari last_ms.
/// Dua lapis: <folder>/*.jsonl (sesi utama) dan <folder>/<dir-sesi>/*.jsonl (subagent).
pub fn collect_sessions(now_ms: i64, max_age_ms: i64, limit_files: usize) -> Vec<SessionInfo> {
    let root = format!("{}/.omp/agent/sessions", crate::home());
    let mut files: Vec<(i64, std::path::PathBuf)> = vec![];
    let push_jsonl = |files: &mut Vec<(i64, std::path::PathBuf)>, f: &std::fs::DirEntry| {
        let p = f.path();
        if p.extension().map(|x| x == "jsonl").unwrap_or(false) {
            let mtime = f
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            if now_ms - mtime < max_age_ms {
                files.push((mtime, p));
            }
        }
    };
    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            if let Ok(rd2) = std::fs::read_dir(e.path()) {
                for f in rd2.flatten() {
                    if f.path().is_dir() {
                        if let Ok(rd3) = std::fs::read_dir(f.path()) {
                            for g in rd3.flatten() {
                                push_jsonl(&mut files, &g);
                            }
                        }
                    } else {
                        push_jsonl(&mut files, &f);
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.truncate(limit_files);
    let mut out: Vec<SessionInfo> = files.iter().filter_map(|(_, p)| parse_session_file(p)).collect();
    enrich_from_stats_db(&mut out);
    out.sort_by(|a, b| b.last_ms.cmp(&a.last_ms));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_parse() {
        // Terverifikasi silang dengan `date`: 08:11 UTC = 1793607060; 08:39:03,805 = +1683,805 dtk.
        assert_eq!(iso_to_ms("2026-11-02T08:39:03.805Z"), Some(1793608743805));
        assert_eq!(iso_to_ms("2026-11-02T00:00:00.000Z"), Some(1793577600000));
    }
}
