//! hq-server — Rizal Agent HQ (Axum).
//! REST: /api/state /api/agents /api/alerts /api/history /api/health
//! POST /api/heartbeat (agent eksternal), POST /api/alerts/:id/ack|snooze
//! Statis: folder web/ (kantor 3D). Penyimpanan: SQLite (snapshot & alert).
//!
//! Env: HQ_PORT (8900), HQ_BIND (127.0.0.1), HQ_INTERVAL (30 dtk), HQ_DATA (data/), HQ_WEB (web/)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use hq_core::timefmt::now_ms;
use hq_core::*;
use serde::Deserialize;
use std::collections::HashMap;
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
            id TEXT PRIMARY KEY, agent TEXT, level TEXT, title TEXT, since INTEGER, state TEXT);",
    )
    .expect("skema SQLite");
    conn
}

async fn refresh(st: &AppState) {
    let prev_agents = st.snap.read().await.agents.clone();
    let mut collect = st.collect.write().await;
    let mut snap = hq_collect::collect_snapshot(&mut collect);
    drop(collect);

    let evaluated = hq_alert::evaluate(&snap, &prev_agents);
    let mut alerts_lock = st.alerts.write().await;
    *alerts_lock = hq_alert::reconcile(alerts_lock.clone(), evaluated);
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
    *st.snap.write().await = snap.clone();
    // Dorong snapshot baru ke semua klien WebSocket (live, tanpa polling).
    if let Ok(text) = serde_json::to_string(&snap) {
        let _ = st.tx.send(text);
    }

    // Simpan snapshot ringkas (± tiap siklus; query riwayat mengambil sampel).
    if let Ok(db) = st.db.lock() {
        let active = snap.agents.iter().filter(|a| a.status == Status::Bekerja).count() as i64;
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
    let name: String = hb.agent.to_lowercase().replace(' ', "-").chars().filter(|c| c.is_alphanumeric() || *c == '-').collect();
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

async fn alert_ack(State(st): State<Arc<AppState>>, Path(id): Path<String>) -> StatusCode {
    let mut alerts = st.alerts.write().await;
    if let Some(a) = alerts.iter_mut().find(|a| a.id == id) {
        a.state = AlertState::Ditangani;
        return StatusCode::OK;
    }
    StatusCode::NOT_FOUND
}

async fn alert_snooze(State(st): State<Arc<AppState>>, Path(id): Path<String>, Json(act): Json<AlertAction>) -> StatusCode {
    let mut alerts = st.alerts.write().await;
    if let Some(a) = alerts.iter_mut().find(|a| a.id == id) {
        a.state = AlertState::Ditunda;
        a.snoozed_until = Some(now_ms() + act.minutes.unwrap_or(60) * 60 * 1000);
        return StatusCode::OK;
    }
    StatusCode::NOT_FOUND
}

#[derive(Deserialize)]
struct HistoryQ {
    days: Option<i64>,
}

async fn get_history(State(st): State<Arc<AppState>>, Query(q): Query<HistoryQ>) -> Json<serde_json::Value> {
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
    let port: u16 = env_or("HQ_PORT", "8900").parse().unwrap_or(8900);
    let bind = env_or("HQ_BIND", "127.0.0.1");
    let interval: u64 = env_or("HQ_INTERVAL", "30").parse().unwrap_or(30);
    let data_dir = env_or("HQ_DATA", "data");
    let web_dir = env_or("HQ_WEB", "web");
    std::fs::create_dir_all(format!("{data_dir}/heartbeats")).ok();

    let db = db_init(&format!("{data_dir}/hq.db"));
    let (tx, _rx0) = tokio::sync::broadcast::channel::<String>(16);
    let mut collect_state = hq_collect::CollectState::default();
    collect_state.heartbeat_dir = format!("{data_dir}/heartbeats");
    let st = Arc::new(AppState {
        snap: RwLock::new(Snapshot::default()),
        alerts: RwLock::new(vec![]),
        collect: RwLock::new(collect_state),
        data_dir: data_dir.clone(),
        db: std::sync::Mutex::new(db),
        tx,
    });

    refresh(&st).await;
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
        .route("/api/state", get(|State(st): State<Arc<AppState>>| async move { Json(st.snap.read().await.clone()) }))
        .route("/api/agents", get(|State(st): State<Arc<AppState>>| async move { Json(serde_json::json!({ "agents": st.snap.read().await.agents })) }))
        .route("/api/alerts", get(|State(st): State<Arc<AppState>>| async move { Json(serde_json::json!({ "alerts": st.alerts.read().await.clone() })) }))
        .route("/api/history", get(get_history))
        .route("/api/health", get(|| async { Json(serde_json::json!({ "ok": true, "at": now_ms(), "app": "rizal-agent-hq" })) }))
        .route("/ws", get(ws_handler))
        .route("/api/heartbeat", post(post_heartbeat))
        .route("/api/alerts/{id}/ack", post(alert_ack))
        .route("/api/alerts/{id}/snooze", post(alert_snooze))
        .fallback_service(ServeDir::new(web_dir.clone()))
        .with_state(st);

    let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}")).await.expect("bind");
    println!("Rizal Agent HQ jalan di http://{bind}:{port}/ (interval {interval}s)");
    axum::serve(listener, app).await.expect("serve");
}
