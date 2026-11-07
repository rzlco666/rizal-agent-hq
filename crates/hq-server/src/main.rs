//! hq-server — Rizal Agent HQ (Axum).
//! REST: /api/state /api/agents /api/alerts /api/history /api/health /api/stocks /api/portfolio
//! /api/knowledge/* (pencarian + halaman + sync + promote) /api/memory* (B.5)
//! Ukur performa: log "[perf] refresh ..." tiap siklus + /api/health diperdalam
//! (DB, saham, memori, disk, durasi refresh terakhir). Lihat docs/PERF.md.
//! POST /api/heartbeat (agent eksternal), POST /api/alerts/:id/ack|snooze,
//! POST /api/stocks/refresh (harga live, hormati gerbang harian)
//! Statis: folder web/ (kantor 3D). Penyimpanan: SQLite (snapshot & alert).
//!
//! Env: HQ_PORT (8900), HQ_BIND (127.0.0.1), HQ_INTERVAL (30 dtk), HQ_DATA (data/), HQ_WEB (web/)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, patch, post},
    Router,
};
use hq_core::timefmt::{now_ms, wib_date};
use hq_core::*;
use serde::Deserialize;

mod notion_knowledge;
mod portfolio;
mod stocks_price;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

struct AppState {
    snap: RwLock<Snapshot>,
    alerts: RwLock<Vec<Alert>>,
    collect: RwLock<hq_collect::CollectState>,
    data_dir: String,
    db: std::sync::Mutex<rusqlite::Connection>,
    tx: tokio::sync::broadcast::Sender<String>,
    /// Cache ringkasan LLM: task_id -> (hash konten, ringkasan).
    llm_cache: Arc<std::sync::Mutex<HashMap<String, (u64, String)>>>,
    llm_pending: Arc<std::sync::Mutex<HashSet<String>>>,
    llm_tx: Option<std::sync::mpsc::Sender<LlmJob>>,
    /// State alert yang dipilih Rizal (ditangani/ditunda), dipulihkan dari SQLite.
    alert_saved: std::sync::Mutex<HashMap<String, (String, Option<i64>)>>,
    /// Harga live saham: klien HTTP, kunci env, cache, dan kunci serialisasi.
    stocks: Arc<stocks_price::StocksHandle>,
    /// Pengetahuan dari Notion: cermin sync + memori B.5 (hanya candidate
    /// yang ditulis sync; trusted hanya lewat promosi manusia).
    notion: Arc<notion_knowledge::NotionHandle>,
    /// Waktu start proses (untuk uptime di /api/health).
    started_at: std::time::Instant,
    /// Hasil ukur siklus refresh terakhir (durasi per kolektor).
    last_perf: RwLock<PerfSnapshot>,
}

/// Hasil ukur satu siklus `refresh()` — diisi tiap siklus, dibaca /api/health.
/// Semua durasi dalam milidetik. Nilai 0 pada kolektor = cache masih segar.
#[derive(Default, Clone)]
struct PerfSnapshot {
    total_ms: u64,
    collect_ms: u64,
    system_ms: u64,
    docker_ms: u64,
    tailscale_ms: u64,
    git_ms: u64,
    processes_ms: u64,
    tmux_ms: u64,
    omp_stats_ms: u64,
    omp_usage_ms: u64,
    sessions_ms: u64,
    sources_ms: u64,
    alert_ms: u64,
    db_ms: u64,
}

/// Satu pekerjaan perapi LLM (K2-C lapisan 4).
#[derive(Debug)]
struct LlmJob {
    task_id: String,
    hash: u64,
    title: String,
    detail: String,
}

fn hash_text(a: &str, b: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for byte in a.bytes().chain(b.bytes()) {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Jalankan `omp -p` satu prompt; None bila gagal/timeout (maks 45 detik).
fn run_omp_prompt(model: &str, prompt: &str) -> Option<String> {
    // PATH server (systemd/nohup) minimal — omp tinggal di ~/.bun/bin,
    // samakan dengan perlakuan kolektor (run_timeout di hq-collect).
    let home = std::env::var("HOME").unwrap_or_default();
    let path = format!(
        "{}:{}/.bun/bin:{}/.local/bin:{}",
        std::env::var("PATH").unwrap_or_default(),
        home,
        home,
        std::env::var("PATH").unwrap_or_default()
    );
    let mut child = std::process::Command::new("omp")
        .env("PATH", path)
        .args(["--model", model, "-p", prompt])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let out = child.wait_with_output().ok()?;
                let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let clean: String = text
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty() && !l.starts_with("Working"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let clean = clean
                    .trim_matches(|c| c == '"' || c == '`')
                    .trim()
                    .to_string();
                if clean.chars().count() < 10 || clean.to_lowercase().starts_with("error") {
                    return None;
                }
                return Some(clean.chars().take(280).collect());
            }
            Ok(None) => {
                if start.elapsed().as_secs() > 45 {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Err(_) => return None,
        }
    }
}

/// Utas pekerja perapi LLM: antrean diproses satu-satu, hasil disimpan ke
/// SQLite + cache. Kegagalan total = ringkasan template tetap dipakai (fallback).
fn llm_worker(
    rx: std::sync::mpsc::Receiver<LlmJob>,
    db_path: String,
    models: Vec<String>,
    cache: Arc<std::sync::Mutex<HashMap<String, (u64, String)>>>,
    pending: Arc<std::sync::Mutex<HashSet<String>>>,
) {
    let db = rusqlite::Connection::open(&db_path).ok();
    if let Some(db) = &db {
        let _ = db.execute_batch("PRAGMA busy_timeout=5000;");
    }
    let mut attempts: HashMap<(String, u64), std::time::Instant> = HashMap::new();
    while let Ok(job) = rx.recv() {
        if let Ok(p) = pending.lock() {
            if !p.contains(&job.task_id) {
                continue;
            }
        }
        let done = |pending: &Arc<std::sync::Mutex<HashSet<String>>>| {
            if let Ok(mut p) = pending.lock() {
                p.remove(&job.task_id);
            }
        };
        // Cooldown 20 menit per (task, hash) agar kegagalan tidak mengantre terus.
        if let Some(t) = attempts.get(&(job.task_id.clone(), job.hash)) {
            if t.elapsed().as_secs() < 1200 {
                done(&pending);
                continue;
            }
        }
        if let Ok(c) = cache.lock() {
            if c.get(&job.task_id)
                .map(|(h, _)| *h == job.hash)
                .unwrap_or(false)
            {
                done(&pending);
                continue;
            }
        }
        attempts.insert((job.task_id.clone(), job.hash), std::time::Instant::now());
        let title: String = job.title.chars().take(1200).collect();
        let detail: String = job.detail.chars().take(300).collect();
        let prompt = format!(
            "Ringkas judul sesi kerja agent berikut menjadi SATU kalimat Bahasa Indonesia yang jelas dan manusiawi, maksimal 25 kata. Jangan mengarang fakta yang tidak ada di teks. Tulis hanya kalimat ringkasannya, tanpa tanda kutip, tanpa awalan apa pun.

Judul asli: {title}
Detail: {detail}"
        );
        let mut result = None;
        let mut used_model = "";
        let t_llm = std::time::Instant::now();
        for m in &models {
            if let Some(r) = run_omp_prompt(m, &prompt) {
                result = Some(r);
                used_model = m;
                break;
            }
        }
        println!(
            "[perf] llm task={} model={} dur_ms={} ok={}",
            job.task_id,
            used_model,
            t_llm.elapsed().as_millis(),
            result.is_some()
        );
        if let Some(text) = result {
            if let Some(db) = &db {
                let _ = db.execute(
                    "INSERT OR REPLACE INTO llm_summaries(task_id, hash, summary, at) VALUES(?1, ?2, ?3, ?4)",
                    rusqlite::params![job.task_id, job.hash as i64, text, now_ms()],
                );
            }
            if let Ok(mut c) = cache.lock() {
                c.insert(job.task_id.clone(), (job.hash, text));
            }
        }
        done(&pending);
    }
}

fn persist_alert(db: &rusqlite::Connection, a: &Alert) {
    match a.state {
        AlertState::Ditangani => {
            let _ = db.execute(
                "INSERT OR REPLACE INTO alert_state(id, state, snoozed_until, updated) VALUES(?1, 'ditangani', NULL, ?2)",
                rusqlite::params![a.id, now_ms()],
            );
        }
        AlertState::Ditunda => {
            let _ = db.execute(
                "INSERT OR REPLACE INTO alert_state(id, state, snoozed_until, updated) VALUES(?1, 'ditunda', ?2, ?3)",
                rusqlite::params![a.id, a.snoozed_until, now_ms()],
            );
        }
        _ => {
            let _ = db.execute(
                "DELETE FROM alert_state WHERE id = ?1",
                rusqlite::params![a.id],
            );
        }
    }
}

fn env_or(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.into())
}

fn db_init(path: &str) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).expect("buka SQLite");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshots(
            at INTEGER PRIMARY KEY, requests INTEGER, input INTEGER, output INTEGER,
            cache INTEGER, cash REAL, shadow REAL, active_agents INTEGER);
         CREATE TABLE IF NOT EXISTS alert_log(
            id TEXT PRIMARY KEY, agent TEXT, level TEXT, title TEXT, since INTEGER, state TEXT);
         CREATE TABLE IF NOT EXISTS alert_state(
            id TEXT PRIMARY KEY, state TEXT, snoozed_until INTEGER, updated INTEGER);
         CREATE TABLE IF NOT EXISTS llm_summaries(
            task_id TEXT PRIMARY KEY, hash INTEGER, summary TEXT, at INTEGER);",
    )
    .expect("skema SQLite");
    portfolio::ensure_portfolio_schema(&conn);
    notion_knowledge::ensure_notion_schema(&conn);
    conn
}

async fn refresh(st: &AppState) {
    let t_total = std::time::Instant::now();
    let prev_agents = st.snap.read().await.agents.clone();
    let t_collect = std::time::Instant::now();
    let mut collect = st.collect.write().await;
    let (mut snap, timings) = hq_collect::collect_snapshot(&mut collect);
    let collect_ms = t_collect.elapsed().as_millis() as u64;
    drop(collect);

    let t_alert = std::time::Instant::now();
    let evaluated = hq_alert::evaluate(&snap, &prev_agents);
    let mut alerts_lock = st.alerts.write().await;
    *alerts_lock = hq_alert::reconcile(alerts_lock.clone(), evaluated);
    // Pulihkan pilihan Rizal dari SQLite: alert yang sudah ditangani/ditunda
    // TIDAK boleh bangkit lagi sebagai aktif hanya karena server restart.
    if let Ok(saved) = st.alert_saved.lock() {
        let now = now_ms();
        for a in alerts_lock.iter_mut() {
            if let Some((state, until)) = saved.get(&a.id) {
                match state.as_str() {
                    "ditangani" => a.state = AlertState::Ditangani,
                    "ditunda" if until.map(|u| u > now).unwrap_or(false) => {
                        a.state = AlertState::Ditunda;
                        a.snoozed_until = *until;
                    }
                    _ => {}
                }
            }
        }
    }
    let alerts = alerts_lock.clone();
    drop(alerts_lock);

    // Sematkan jumlah alert aktif per agent + lampirkan alert ke snapshot.
    let mut counts: HashMap<String, u32> = HashMap::new();
    for a in alerts.iter().filter(|a| a.state == AlertState::Aktif) {
        *counts.entry(a.agent_id.clone()).or_default() += 1;
    }
    for ag in snap.agents.iter_mut() {
        ag.active_alerts = counts.get(&ag.id).copied().unwrap_or(0);
        if ag.active_alerts > 0 && ag.status == Status::Bekerja {
            // Tetap Bekerja; status MenungguRizal hanya dari bukti sesi menunggu, bukan alert kuota.
        }
    }
    snap.alerts = alerts;
    let alert_ms = t_alert.elapsed().as_millis() as u64;
    // Perapi LLM (K2-C lapisan 4): judul sesi mentah yang panjang dirapikan
    // model $0 di latar; selama belum siap, template tetap tampil (fallback).
    if let Some(tx) = &st.llm_tx {
        for t in snap.tasks.iter_mut() {
            if t.title.chars().count() < 40 {
                continue;
            }
            // Hash dari JUDUL saja: detail sesi berubah tiap siklus (jumlah
            // panggilan perkakas bertambah) dan akan menggagalkan cache.
            let h = hash_text(&t.title, "");
            let cached = st.llm_cache.lock().ok().and_then(|c| c.get(&t.id).cloned());
            match cached {
                Some((ch, text)) if ch == h => t.summary = Some(text),
                _ => {
                    let fresh = st
                        .llm_pending
                        .lock()
                        .map(|mut p| p.insert(t.id.clone()))
                        .unwrap_or(false);
                    if fresh {
                        let _ = tx.send(LlmJob {
                            task_id: t.id.clone(),
                            hash: h,
                            title: t.title.clone(),
                            detail: t.detail.clone(),
                        });
                    }
                }
            }
        }
    }
    *st.snap.write().await = snap.clone();
    // Dorong snapshot baru ke semua klien WebSocket (live, tanpa polling).
    if let Ok(text) = serde_json::to_string(&snap) {
        let _ = st.tx.send(text);
    }

    // Simpan snapshot ringkas (± tiap siklus; query riwayat mengambil sampel).
    let t_db = std::time::Instant::now();
    if let Ok(db) = st.db.lock() {
        for a in &snap.alerts {
            persist_alert(&db, a);
        }
        if let Ok(mut saved) = st.alert_saved.lock() {
            saved.clear();
            for a in &snap.alerts {
                match a.state {
                    AlertState::Ditangani => {
                        saved.insert(a.id.clone(), ("ditangani".into(), None));
                    }
                    AlertState::Ditunda => {
                        saved.insert(a.id.clone(), ("ditunda".into(), a.snoozed_until));
                    }
                    _ => {}
                }
            }
        }
        let active = snap
            .agents
            .iter()
            .filter(|a| a.status == Status::Bekerja)
            .count() as i64;
        let _ = db.execute(
            "INSERT OR REPLACE INTO snapshots(at,requests,input,output,cache,cash,shadow,active_agents) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                snap.generated_at,
                snap.total_usage.requests as i64,
                snap.total_usage.input_tokens as i64,
                snap.total_usage.output_tokens as i64,
                snap.total_usage.cache_tokens as i64,
                snap.total_usage.cash_usd,
                snap.total_usage.shadow_usd,
                active
            ],
        );
    }
    let db_ms = t_db.elapsed().as_millis() as u64;
    let total_ms = t_total.elapsed().as_millis() as u64;
    *st.last_perf.write().await = PerfSnapshot {
        total_ms,
        collect_ms,
        system_ms: timings.system_ms,
        docker_ms: timings.docker_ms,
        tailscale_ms: timings.tailscale_ms,
        git_ms: timings.git_ms,
        processes_ms: timings.processes_ms,
        tmux_ms: timings.tmux_ms,
        omp_stats_ms: timings.omp_stats_ms,
        omp_usage_ms: timings.omp_usage_ms,
        sessions_ms: timings.sessions_ms,
        sources_ms: timings.sources_ms,
        alert_ms,
        db_ms,
    };
    println!(
        "[perf] refresh total={total_ms}ms collect={collect_ms}ms sys={}ms docker={}ms ts={}ms git={}ms ps={}ms tmux={}ms omp_stats={}ms omp_usage={}ms sessions={}ms sources={}ms alert={alert_ms}ms db={db_ms}ms",
        timings.system_ms,
        timings.docker_ms,
        timings.tailscale_ms,
        timings.git_ms,
        timings.processes_ms,
        timings.tmux_ms,
        timings.omp_stats_ms,
        timings.omp_usage_ms,
        timings.sessions_ms,
        timings.sources_ms,
    );
}

#[derive(Deserialize)]
struct Heartbeat {
    agent: String,
    role: Option<String>,
    status: Option<String>,
    task: Option<String>,
}

async fn post_heartbeat(State(st): State<Arc<AppState>>, Json(hb): Json<Heartbeat>) -> StatusCode {
    let dir = format!("{}/heartbeats", st.data_dir);
    let _ = std::fs::create_dir_all(&dir);
    let name: String = hb
        .agent
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();
    if name.is_empty() {
        return StatusCode::BAD_REQUEST;
    }
    let body = serde_json::json!({
        "agent": hb.agent,
        "role": hq_core::redact::redact(hb.role.as_deref().unwrap_or("")),
        "status": hb.status.unwrap_or_else(|| "bekerja".into()),
        "task": hq_core::redact::redact(hb.task.as_deref().unwrap_or("")),
        "at": now_ms(),
    });
    match std::fs::write(format!("{dir}/{name}.json"), body.to_string()) {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[derive(Deserialize)]
struct AlertAction {
    minutes: Option<i64>,
}

/// Setelah aksi Rizal: snapshot & klien diperbarui SEKETIKA (tidak menunggu
/// siklus refresh 30 detik) dan state disimpan ke SQLite.
async fn apply_alert_change(st: &AppState) {
    let alerts = st.alerts.read().await.clone();
    {
        let mut snap = st.snap.write().await;
        snap.alerts = alerts.clone();
        if let Ok(text) = serde_json::to_string(&*snap) {
            let _ = st.tx.send(text);
        }
    }
    if let Ok(db) = st.db.lock() {
        for a in &alerts {
            persist_alert(&db, a);
        }
    }
    if let Ok(mut saved) = st.alert_saved.lock() {
        saved.clear();
        for a in &alerts {
            match a.state {
                AlertState::Ditangani => {
                    saved.insert(a.id.clone(), ("ditangani".into(), None));
                }
                AlertState::Ditunda => {
                    saved.insert(a.id.clone(), ("ditunda".into(), a.snoozed_until));
                }
                _ => {}
            }
        }
    }
}

async fn alert_ack(State(st): State<Arc<AppState>>, Path(id): Path<String>) -> StatusCode {
    {
        let mut alerts = st.alerts.write().await;
        match alerts.iter_mut().find(|a| a.id == id) {
            Some(a) => a.state = AlertState::Ditangani,
            None => return StatusCode::NOT_FOUND,
        }
    }
    apply_alert_change(&st).await;
    StatusCode::OK
}

async fn alert_snooze(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(act): Json<AlertAction>,
) -> StatusCode {
    {
        let mut alerts = st.alerts.write().await;
        match alerts.iter_mut().find(|a| a.id == id) {
            Some(a) => {
                a.state = AlertState::Ditunda;
                a.snoozed_until =
                    Some(now_ms() + act.minutes.unwrap_or(60).clamp(5, 10080) * 60 * 1000);
            }
            None => return StatusCode::NOT_FOUND,
        }
    }
    apply_alert_change(&st).await;
    StatusCode::OK
}

/// Saham: portofolio dari file milik user di {data_dir}/stocks.json.
/// File tidak ada = portofolio kosong (bukan data contoh). File rusak = error jujur, bukan angka palsu.
/// `quotes` hanya kolom tampilan harga live — harga file & total portofolio tidak berubah.
async fn get_stocks(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let path = format!("{}/stocks.json", st.data_dir);
    let (stocks, source, error) = match stocks_price::read_stocks(&path) {
        stocks_price::StocksFile::Missing => (vec![], "none", None),
        stocks_price::StocksFile::Invalid(e) => (vec![], "file", Some(e)),
        stocks_price::StocksFile::Ok(v) => (v, "file", None),
    };
    let (quotes, updated) = st.stocks.quotes_view();
    Json(serde_json::json!({
        "stocks": stocks,
        "source": source,
        "error": error,
        "quotes": quotes,
        "quotes_updated_at": updated,
    }))
}

/// POST /api/stocks/refresh — mencoba ambil harga sekarang juga, tetap menaati
/// gerbang sekali-sehari, akhir pekan, dan kuota. Balasan: {refreshed, reason}.
async fn post_stocks_refresh(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(st.stocks.trigger(true).await)
}

#[derive(Deserialize)]
struct HistoryQ {
    days: Option<i64>,
}

async fn get_history(
    State(st): State<Arc<AppState>>,
    Query(q): Query<HistoryQ>,
) -> Json<serde_json::Value> {
    let days = q.days.unwrap_or(7);
    let since = now_ms() - days * 86400 * 1000;
    let mut rows = vec![];
    if let Ok(db) = st.db.lock() {
        if let Ok(mut stmt) = db.prepare("SELECT at,requests,input,output,cache,cash,shadow,active_agents FROM snapshots WHERE at>=?1 ORDER BY at") {
            if let Ok(iter) = stmt.query_map(rusqlite::params![since], |r| {
                Ok(serde_json::json!({
                    "at": r.get::<_, i64>(0)?, "requests": r.get::<_, i64>(1)?,
                    "input": r.get::<_, i64>(2)?, "output": r.get::<_, i64>(3)?,
                    "cache": r.get::<_, i64>(4)?, "cash": r.get::<_, f64>(5)?,
                    "shadow": r.get::<_, f64>(6)?, "activeAgents": r.get::<_, i64>(7)?,
                }))
            }) {
                rows = iter.flatten().collect();
            }
        }
    }
    Json(serde_json::json!({ "history": rows }))
}

/// Baca VmRSS + VmHWM (puncak) proses sendiri dari /proc/self/status (kB -> MB).
fn parse_mem_status(text: &str) -> (Option<f64>, Option<f64>) {
    let get = |k: &str| -> Option<f64> {
        text.lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
            .map(|kb| (kb / 1024.0 * 10.0).round() / 10.0)
    };
    (get("VmRSS:"), get("VmHWM:"))
}

fn read_self_mem() -> (Option<f64>, Option<f64>) {
    std::fs::read_to_string("/proc/self/status")
        .map(|t| parse_mem_status(&t))
        .unwrap_or((None, None))
}

/// "2026-11-07T15:00:00+07:00" (format `iso_wib`) -> epoch ms sejati.
/// `iso_to_ms` menghitung waktu dinding sebagai UTC, jadi koreksi -7 jam.
fn wib_iso_to_ms(s: &str) -> Option<i64> {
    hq_collect::sessions::iso_to_ms(s).map(|ms| ms - 7 * 3600 * 1000)
}

/// GET /api/health — diperdalam (riset performa 2026-11-07, M3): liveness di
/// `ok`, plus DB, saham, memori, disk, dan hasil ukur refresh terakhir.
/// Tidak memanggil subprocess apa pun; DB dicek via try_lock (sibuk = ok:false).
async fn get_health(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let now = now_ms();
    let today = wib_date(now);
    let (db_ok, db_err, snap_rows, llm_rows) = match st.db.try_lock() {
        Ok(db) => {
            let snaps: Result<i64, _> =
                db.query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0));
            let llms: Result<i64, _> =
                db.query_row("SELECT COUNT(*) FROM llm_summaries", [], |r| r.get(0));
            match (snaps, llms) {
                (Ok(s), Ok(l)) => (true, None, Some(s), Some(l)),
                _ => (false, Some("query gagal".to_string()), None, None),
            }
        }
        Err(_) => (false, Some("db sibuk".to_string()), None, None),
    };
    let cache = stocks_price::load_cache(&st.data_dir);
    let mut fresh = 0usize;
    let mut max_fetch: Option<(i64, String, String)> = None; // (epoch, fetched_at, source)
    for q in cache.symbols.values() {
        if q.last_fetch_wib_date == today {
            fresh += 1;
        }
        if let Some(ms) = wib_iso_to_ms(&q.fetched_at) {
            if max_fetch.as_ref().map(|(m, _, _)| ms > *m).unwrap_or(true) {
                max_fetch = Some((ms, q.fetched_at.clone(), q.source.clone()));
            }
        }
    }
    let (rss_mb, peak_mb) = read_self_mem();
    let perf = st.last_perf.read().await.clone();
    let disk_percent = st.snap.read().await.env.disk_percent;
    Json(serde_json::json!({
        "ok": true,
        "at": now,
        "app": "rizal-agent-hq",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": st.started_at.elapsed().as_secs(),
        "db": {
            "ok": db_ok,
            "error": db_err,
            "snapshots": snap_rows,
            "llm_summaries": llm_rows,
        },
        "stocks": {
            "fresh": fresh > 0,
            "symbols": cache.symbols.len(),
            "fresh_symbols": fresh,
            "age_secs": max_fetch.as_ref().map(|(ms, _, _)| (now - ms).max(0) / 1000),
            "last_fetch": max_fetch.as_ref().map(|(_, f, _)| f.clone()),
            "source": max_fetch.as_ref().map(|(_, _, s)| s.clone()),
        },
        "mem": { "rss_mb": rss_mb, "peak_mb": peak_mb },
        "disk": { "percent": disk_percent },
        "perf": {
            "refresh_total_ms": perf.total_ms,
            "collect_ms": perf.collect_ms,
            "system_ms": perf.system_ms,
            "docker_ms": perf.docker_ms,
            "tailscale_ms": perf.tailscale_ms,
            "git_ms": perf.git_ms,
            "processes_ms": perf.processes_ms,
            "tmux_ms": perf.tmux_ms,
            "omp_stats_ms": perf.omp_stats_ms,
            "omp_usage_ms": perf.omp_usage_ms,
            "sessions_ms": perf.sessions_ms,
            "sources_ms": perf.sources_ms,
            "alert_ms": perf.alert_ms,
            "db_ms": perf.db_ms,
        },
    }))
}

/// GET /api/portfolio — posisi + hitung dari cache (satu-satunya sumber hitung;
/// JS hanya menampilkan). `stocks_error`/`portfolio_error` mengikuti pola
/// `get_stocks`: file rusak = error jujur, bukan angka palsu.
async fn get_portfolio(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let today = wib_date(st.stocks.now_ms());
    let (stocks, stocks_error) =
        match stocks_price::read_stocks(&format!("{}/stocks.json", st.data_dir)) {
            stocks_price::StocksFile::Ok(v) => (v, None),
            stocks_price::StocksFile::Missing => (vec![], None),
            stocks_price::StocksFile::Invalid(e) => (vec![], Some(e)),
        };
    let (pf, pf_error) = match portfolio::read_portfolio(&portfolio::portfolio_path(&st.data_dir)) {
        portfolio::PortfolioRead::Ok(v) => (v, None),
        portfolio::PortfolioRead::Missing => (portfolio::PortfolioFile::default(), None),
        portfolio::PortfolioRead::Invalid(e) => (portfolio::PortfolioFile::default(), Some(e)),
    };
    let cache = stocks_price::load_cache(&st.data_dir);
    let view = portfolio::compute(&pf, &stocks, &cache, &today);
    let mut out = serde_json::to_value(&view).unwrap_or(serde_json::json!({}));
    out["stocks_error"] = serde_json::json!(stocks_error);
    out["portfolio_error"] = serde_json::json!(pf_error);
    Json(out)
}

#[derive(Deserialize)]
struct HoldingBody {
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    shares: Option<f64>,
    #[serde(default)]
    lots: Option<f64>,
    #[serde(default)]
    avg_buy_price: Option<f64>,
    #[serde(default)]
    bought_at: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

fn load_stocks_for_validation(data_dir: &str) -> Vec<stocks_price::Stock> {
    match stocks_price::read_stocks(&format!("{data_dir}/stocks.json")) {
        stocks_price::StocksFile::Ok(v) => v,
        _ => vec![],
    }
}

/// POST /api/portfolio/holdings — tambah posisi. Simbol ganda = 409 (pakai PATCH).
async fn post_portfolio_holding(
    State(st): State<Arc<AppState>>,
    Json(body): Json<HoldingBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let today = wib_date(st.stocks.now_ms());
    let stocks = load_stocks_for_validation(&st.data_dir);
    let input = portfolio::HoldingInput {
        symbol: body.symbol,
        shares: body.shares,
        lots: body.lots,
        avg_buy_price: body.avg_buy_price,
        bought_at: body.bought_at,
        note: body.note,
    };
    let holding = match portfolio::validate_holding(&input, &stocks, &today) {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    };
    let path = portfolio::portfolio_path(&st.data_dir);
    let mut pf = match portfolio::read_portfolio(&path) {
        portfolio::PortfolioRead::Ok(v) => v,
        portfolio::PortfolioRead::Missing => portfolio::PortfolioFile::default(),
        portfolio::PortfolioRead::Invalid(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    if pf
        .holdings
        .iter()
        .any(|h| portfolio::norm_symbol(&h.symbol) == portfolio::norm_symbol(&holding.symbol))
    {
        return (
            StatusCode::CONFLICT,
            Json(
                serde_json::json!({ "error": format!("Simbol {} sudah ada — ubah lewat PATCH.", holding.symbol) }),
            ),
        );
    }
    pf.holdings.push(holding.clone());
    if let Err(e) = portfolio::save_portfolio(&path, &pf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        );
    }
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "holding": holding })),
    )
}

/// PATCH /api/portfolio/holdings/:symbol — ubah posisi. Field yang tidak
/// diisi dipertahankan; `avg_buy_price` tidak bisa dikosongkan lewat PATCH
/// (hapus lalu tambah ulang bila perlu).
async fn patch_portfolio_holding(
    State(st): State<Arc<AppState>>,
    Path(symbol): Path<String>,
    Json(body): Json<HoldingBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let today = wib_date(st.stocks.now_ms());
    let stocks = load_stocks_for_validation(&st.data_dir);
    let path = portfolio::portfolio_path(&st.data_dir);
    let mut pf = match portfolio::read_portfolio(&path) {
        portfolio::PortfolioRead::Ok(v) => v,
        portfolio::PortfolioRead::Missing => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Belum ada posisi." })),
            );
        }
        portfolio::PortfolioRead::Invalid(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    let idx = match pf
        .holdings
        .iter()
        .position(|h| portfolio::norm_symbol(&h.symbol) == portfolio::norm_symbol(&symbol))
    {
        Some(i) => i,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({ "error": format!("Simbol {symbol} belum ada di portofolio.") }),
                ),
            );
        }
    };
    let old = pf.holdings[idx].clone();
    let merged = portfolio::HoldingInput {
        symbol: Some(old.symbol.clone()),
        shares: body.shares.or(if body.lots.is_some() {
            None
        } else {
            Some(old.shares)
        }),
        lots: body.lots,
        avg_buy_price: body.avg_buy_price.or(old.avg_buy_price),
        bought_at: body.bought_at.or(old.bought_at),
        note: body.note.or(old.note),
    };
    let holding = match portfolio::validate_holding(&merged, &stocks, &today) {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    };
    pf.holdings[idx] = holding.clone();
    if let Err(e) = portfolio::save_portfolio(&path, &pf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({ "holding": holding })),
    )
}

/// DELETE /api/portfolio/holdings/:symbol — hapus posisi. Snapshot histori
/// TIDAK dihapus (agregat masa lalu tetap apa adanya).
async fn delete_portfolio_holding(
    State(st): State<Arc<AppState>>,
    Path(symbol): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let path = portfolio::portfolio_path(&st.data_dir);
    let mut pf = match portfolio::read_portfolio(&path) {
        portfolio::PortfolioRead::Ok(v) => v,
        portfolio::PortfolioRead::Missing => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Belum ada posisi." })),
            );
        }
        portfolio::PortfolioRead::Invalid(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    let idx = match pf
        .holdings
        .iter()
        .position(|h| portfolio::norm_symbol(&h.symbol) == portfolio::norm_symbol(&symbol))
    {
        Some(i) => i,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({ "error": format!("Simbol {symbol} belum ada di portofolio.") }),
                ),
            );
        }
    };
    let removed = pf.holdings.remove(idx);
    if let Err(e) = portfolio::save_portfolio(&path, &pf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({ "deleted": removed.symbol })),
    )
}

#[derive(Deserialize)]
struct PortfolioHistoryQ {
    days: Option<i64>,
    currency: Option<String>,
}

/// GET /api/portfolio/history?days=30&currency=IDR — baris snapshot harian.
async fn get_portfolio_history(
    State(st): State<Arc<AppState>>,
    Query(q): Query<PortfolioHistoryQ>,
) -> Json<serde_json::Value> {
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let now = st.stocks.now_ms();
    let rows = match st.db.lock() {
        Ok(db) => portfolio::read_history(&db, days, q.currency.as_deref(), now),
        Err(_) => vec![],
    };
    Json(serde_json::json!({ "history": rows }))
}

/// POST /api/portfolio/snapshot — trigger manual snapshot hari itu (idempotent).
async fn post_portfolio_snapshot(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let now = st.stocks.now_ms();
    let out = match st.db.lock() {
        Ok(db) => portfolio::take_snapshot(&db, &st.data_dir, now),
        Err(_) => serde_json::json!({ "snapshotted": false, "reason": "database terkunci" }),
    };
    Json(out)
}

#[derive(Deserialize)]
struct FxBody {
    #[serde(default)]
    usd_idr_rate: Option<f64>,
    #[serde(default)]
    rate_asof: Option<String>,
}

/// POST /api/portfolio/config — isi/ubah kurs manual USD→IDR (berpasangan
/// dengan tanggalnya; tanpa kurs, tidak ada grand total IDR).
async fn post_portfolio_config(
    State(st): State<Arc<AppState>>,
    Json(body): Json<FxBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let today = wib_date(st.stocks.now_ms());
    if let Some(r) = body.usd_idr_rate {
        if r <= 0.0 || !r.is_finite() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Kurs harus lebih dari 0." })),
            );
        }
    }
    if let Some(d) = body.rate_asof.as_deref() {
        let d = d.trim();
        if !portfolio::valid_date(d) {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "error": "Tanggal kurs harus format YYYY-MM-DD yang valid." }),
                ),
            );
        }
        if d > today.as_str() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Tanggal kurs tidak boleh di masa depan." })),
            );
        }
    }
    if body.usd_idr_rate.is_some() != body.rate_asof.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Kurs dan tanggal kurs harus diisi bersamaan." })),
        );
    }
    let path = portfolio::portfolio_path(&st.data_dir);
    let mut pf = match portfolio::read_portfolio(&path) {
        portfolio::PortfolioRead::Ok(v) => v,
        portfolio::PortfolioRead::Missing => portfolio::PortfolioFile::default(),
        portfolio::PortfolioRead::Invalid(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    pf.usd_idr_rate = body.usd_idr_rate;
    pf.rate_asof = body.rate_asof.map(|d| d.trim().to_string());
    if let Err(e) = portfolio::save_portfolio(&path, &pf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({ "usd_idr_rate": pf.usd_idr_rate, "rate_asof": pf.rate_asof })),
    )
}

async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(st): State<Arc<AppState>>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        let mut rx = st.tx.subscribe();
        if let Ok(text) = serde_json::to_string(&*st.snap.read().await) {
            let _ = socket.send(axum::extract::ws::Message::Text(text.into())).await;
        }
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(text) => {
                            if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() { break; }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
                incoming = socket.recv() => {
                    match incoming {
                        Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                        Some(Err(_)) => break,
                        _ => {}
                    }
                }
            }
        }
    })
}

#[tokio::main]
async fn main() {
    // Profiling tokio-console (fitur `console`, hanya bila HQ_CONSOLE=1):
    // butuh build dengan RUSTFLAGS="--cfg tokio_unstable" supaya tokio
    // mengemisikan instrumentasi task. Jangan aktifkan di service live.
    #[cfg(feature = "console")]
    if env_or("HQ_CONSOLE", "") == "1" {
        console_subscriber::init();
    }
    let port: u16 = env_or("HQ_PORT", "8900").parse().unwrap_or(8900);
    let bind = env_or("HQ_BIND", "127.0.0.1");
    let interval: u64 = env_or("HQ_INTERVAL", "6").parse().unwrap_or(30);
    let data_dir = env_or("HQ_DATA", "data");
    let web_dir = env_or("HQ_WEB", "web");
    std::fs::create_dir_all(format!("{data_dir}/heartbeats")).ok();

    let db_path = format!("{data_dir}/hq.db");
    let db = db_init(&db_path);
    // Muat ulang state persisten: ringkasan LLM + pilihan alert Rizal.
    let llm_cache = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let llm_pending = Arc::new(std::sync::Mutex::new(HashSet::new()));
    let mut alert_saved_map: HashMap<String, (String, Option<i64>)> = HashMap::new();
    {
        if let Ok(mut stmt) = db.prepare("SELECT task_id, hash, summary FROM llm_summaries") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            }) {
                for row in rows.flatten() {
                    if let Ok(mut c) = llm_cache.lock() {
                        c.insert(row.0, (row.1 as u64, row.2));
                    }
                }
            }
        }
        if let Ok(mut stmt) = db.prepare("SELECT id, state, snoozed_until FROM alert_state") {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                ))
            }) {
                for row in rows.flatten() {
                    alert_saved_map.insert(row.0, (row.1, row.2));
                }
            }
        }
    }
    // Perapi LLM aktif kecuali HQ_LLM=off. Rantai model: $0 dulu, lalu flash murah.
    let llm_tx = if env_or("HQ_LLM", "on") == "off" {
        None
    } else {
        let models: Vec<String> = env_or(
            "HQ_LLM_MODELS",
            "opencode-go/beam-free,opencode-go/glm-5.3-flash",
        )
        .split(',')
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect();
        let (txl, rxl) = std::sync::mpsc::channel::<LlmJob>();
        let (cache2, pending2, path2) = (llm_cache.clone(), llm_pending.clone(), db_path.clone());
        std::thread::spawn(move || llm_worker(rxl, path2, models, cache2, pending2));
        Some(txl)
    };
    let (tx, _rx0) = tokio::sync::broadcast::channel::<String>(16);
    let collect_state = hq_collect::CollectState {
        heartbeat_dir: format!("{data_dir}/heartbeats"),
        ..Default::default()
    };
    let st = Arc::new(AppState {
        snap: RwLock::new(Snapshot::default()),
        alerts: RwLock::new(vec![]),
        collect: RwLock::new(collect_state),
        data_dir: data_dir.clone(),
        db: std::sync::Mutex::new(db),
        tx,
        llm_cache,
        llm_pending,
        llm_tx,
        alert_saved: std::sync::Mutex::new(alert_saved_map),
        stocks: stocks_price::StocksHandle::from_env(&data_dir),
        notion: notion_knowledge::NotionHandle::from_env(&data_dir),
        started_at: std::time::Instant::now(),
        last_perf: RwLock::new(PerfSnapshot::default()),
    });

    refresh(&st).await;
    stocks_price::spawn_scheduler(st.stocks.clone());
    notion_knowledge::spawn_scheduler(st.notion.clone());
    portfolio::spawn_portfolio_scheduler(st.clone());
    {
        let st2 = st.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval.max(10)));
            loop {
                tick.tick().await;
                refresh(&st2).await;
            }
        });
    }

    let app = Router::new()
        .route(
            "/api/state",
            get(
                |State(st): State<Arc<AppState>>| async move { Json(st.snap.read().await.clone()) },
            ),
        )
        .route(
            "/api/agents",
            get(|State(st): State<Arc<AppState>>| async move {
                Json(serde_json::json!({ "agents": st.snap.read().await.agents }))
            }),
        )
        .route(
            "/api/alerts",
            get(|State(st): State<Arc<AppState>>| async move {
                Json(serde_json::json!({ "alerts": st.alerts.read().await.clone() }))
            }),
        )
        .route("/api/history", get(get_history))
        .route("/api/stocks", get(get_stocks))
        .route("/api/stocks/refresh", post(post_stocks_refresh))
        .route("/api/portfolio", get(get_portfolio))
        .route("/api/portfolio/holdings", post(post_portfolio_holding))
        .route(
            "/api/portfolio/holdings/{symbol}",
            patch(patch_portfolio_holding).delete(delete_portfolio_holding),
        )
        .route("/api/portfolio/history", get(get_portfolio_history))
        .route("/api/portfolio/snapshot", post(post_portfolio_snapshot))
        .route("/api/portfolio/config", post(post_portfolio_config))
        .route(
            "/api/knowledge/search",
            get(notion_knowledge::get_knowledge_search),
        )
        .route(
            "/api/knowledge/pages",
            get(notion_knowledge::get_knowledge_pages),
        )
        .route(
            "/api/knowledge/page/{id}",
            get(notion_knowledge::get_knowledge_page),
        )
        .route(
            "/api/knowledge/sync/status",
            get(notion_knowledge::get_knowledge_sync_status),
        )
        .route(
            "/api/knowledge/sync",
            post(notion_knowledge::post_knowledge_sync),
        )
        .route(
            "/api/knowledge/promote",
            post(notion_knowledge::post_knowledge_promote),
        )
        .route(
            "/api/memory",
            get(notion_knowledge::get_memory).post(notion_knowledge::post_memory),
        )
        .route(
            "/api/memory/{id}/promote",
            post(notion_knowledge::post_memory_promote),
        )
        .route(
            "/api/memory/{id}/reject",
            post(notion_knowledge::post_memory_reject),
        )
        .route(
            "/api/memory/{id}/correct",
            post(notion_knowledge::post_memory_correct),
        )
        .route(
            "/api/memory/{id}/delete",
            post(notion_knowledge::post_memory_delete),
        )
        .route("/api/health", get(get_health))
        .route("/ws", get(ws_handler))
        .route("/api/heartbeat", post(post_heartbeat))
        .route("/api/alerts/{id}/ack", post(alert_ack))
        .route("/api/alerts/{id}/snooze", post(alert_snooze))
        .fallback_service(ServeDir::new(web_dir.clone()))
        .with_state(st);

    let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}"))
        .await
        .expect("bind");
    println!("Rizal Agent HQ jalan di http://{bind}:{port}/ (interval {interval}s)");
    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mem_status_membaca_rss_dan_peak() {
        let text =
            "Name:\trizal-agent-hq\nVmRSS:\t172032 kB\nVmHWM:\t1048576 kB\nVmSize:\t123456 kB\n";
        let (rss, peak) = parse_mem_status(text);
        assert_eq!(rss, Some(168.0));
        assert_eq!(peak, Some(1024.0));
        assert_eq!(parse_mem_status("tidak ada baris memori"), (None, None));
    }

    #[test]
    fn wib_iso_to_ms_balik_iso_wib() {
        // iso_wib berpresisi detik, jadi round-trip hanya berlaku untuk
        // nilai kelipatan 1000; milidetik terpotong (fetched_at memang per detik).
        for t in [0i64, 1_793_954_460_000, 1_794_000_000_000] {
            assert_eq!(
                wib_iso_to_ms(&stocks_price::iso_wib(t)),
                Some(t),
                "round-trip gagal untuk {t}"
            );
        }
        assert_eq!(
            wib_iso_to_ms(&stocks_price::iso_wib(1_794_000_000_123)),
            Some(1_794_000_000_000)
        );
        assert_eq!(wib_iso_to_ms("bukan-iso"), None);
    }
}
