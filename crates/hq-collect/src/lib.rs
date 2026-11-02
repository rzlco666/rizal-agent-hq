//! hq-collect — kolektor per SUMBER, read-only & defensif.
//!
//! ARSITEKTUR DINAMIS (registry sumber): setiap sumber data adalah satu
//! fungsi `fn(&mut CollectState, &Probes) -> SourceOut` yang didaftarkan
//! di `registry()`. Menambah konektor/sumber baru = tulis satu fungsi +
//! satu baris registrasi (panduan: docs/MENAMBAH-SUMBER.md). Sumber bisa
//! dimatikan/dinyalakan tanpa kompilasi ulang lewat env:
//!   HQ_SOURCES=omp,cursor,Muse   (daftar putih; kosong = semua)
//!   HQ_DISABLE_SOURCES=mcp         (daftar hitam)
//! Kegagalan satu sumber tidak menjatuhkan snapshot. Semua teks bebas disensor.

pub mod omp;
pub mod sessions;

use hq_core::humanize::{humanize, RawActivity};
use hq_core::redact::redact;
use hq_core::timefmt::now_ms;
use hq_core::*;
use std::collections::HashMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Jalankan perintah dengan batas waktu; None bila gagal/timeout.
pub fn run_timeout(cmd: &str, args: &[&str], timeout_ms: u64) -> Option<String> {
    let mut child = Command::new(cmd)
        .args(args)
        .env(
            "PATH",
            format!(
                "{}:{}/.bun/bin:{}/.local/bin:{}",
                std::env::var("PATH").unwrap_or_default(),
                home(),
                home(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let mut buf = String::new();
                child.stdout.take()?.read_to_string(&mut buf).ok()?;
                return Some(buf);
            }
            Ok(None) => {
                if start.elapsed() > Duration::from_millis(timeout_ms) {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return None,
        }
    }
}

pub fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/home/rizal".into())
}

#[derive(Debug, Clone, Default)]
pub struct Proc {
    pub pid: u32,
    pub cpu: f64,
    pub mem: f64,
    pub etime: String,
    pub comm: String,
    pub args: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcKind {
    Omp,
    Cursor,
    ClaudeCode,
    Codex,
    Devin,
    Docker,
    Gh,
    Tailscale,
    Mcp,
    DevServer,
}

pub fn classify(p: &Proc) -> Option<ProcKind> {
    let hay = format!("{} {}", p.comm, p.args);
    let has = |s: &str| hay.contains(s);
    if has("/omp") || hay.starts_with("omp") || has("oh-my-pi") || has("pi-coding-agent") {
        Some(ProcKind::Omp)
    } else if has("cursor-server") || has("cursor-agent") || has(".cursor/") {
        Some(ProcKind::Cursor)
    } else if has("claude-code") || has("@anthropic-ai/claude") {
        Some(ProcKind::ClaudeCode)
    } else if has("codex") {
        Some(ProcKind::Codex)
    } else if has("devin") {
        Some(ProcKind::Devin)
    } else if has("dockerd") || has("containerd") || has("docker-proxy") {
        Some(ProcKind::Docker)
    } else if p.comm == "gh" || has("gh run watch") {
        Some(ProcKind::Gh)
    } else if has("tailscaled") {
        Some(ProcKind::Tailscale)
    } else if has("mcp") || has("modelcontextprotocol") || has("codegraph") {
        Some(ProcKind::Mcp)
    } else if has("vite") || has("next-server") || has("cargo run") || has("cargo build") || has("cargo watch") {
        Some(ProcKind::DevServer)
    } else {
        None
    }
}

pub fn collect_processes() -> Vec<Proc> {
    let mut out = Vec::new();
    if let Some(txt) = run_timeout("ps", &["-eo", "pid=,pcpu=,pmem=,etime=,comm=,args="], 6000) {
        for line in txt.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let toks: Vec<&str> = line.split_whitespace().collect();
            if toks.len() < 6 {
                continue;
            }
            out.push(Proc {
                pid: toks[0].parse().unwrap_or(0),
                cpu: toks[1].parse().unwrap_or(0.0),
                mem: toks[2].parse().unwrap_or(0.0),
                etime: toks[3].to_string(),
                comm: toks[4].to_string(),
                args: redact(&toks[5..].join(" ")).chars().take(300).collect(),
            });
        }
    }
    out
}

fn read_first_line(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()?.lines().next().map(|s| s.to_string())
}

#[derive(Default)]
pub struct SystemInfo {
    pub hostname: String,
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub disk_percent: f64,
    pub uptime_sec: u64,
}

pub fn collect_system(prev_cpu: &mut Option<(u64, u64)>) -> SystemInfo {
    let mut s = SystemInfo {
        hostname: read_first_line("/etc/hostname").unwrap_or_default(),
        ..Default::default()
    };
    if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
        if let Some(line) = stat.lines().next() {
            let nums: Vec<u64> = line.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
            if nums.len() >= 5 {
                let idle = nums[3] + nums[4];
                let total: u64 = nums.iter().sum();
                if let Some((pi, pt)) = *prev_cpu {
                    let dt = total.saturating_sub(pt);
                    let di = idle.saturating_sub(pi);
                    if dt > 0 {
                        s.cpu_percent = ((1.0 - di as f64 / dt as f64) * 1000.0).round() / 10.0;
                    }
                }
                *prev_cpu = Some((idle, total));
            }
        }
    }
    if let Ok(mem) = std::fs::read_to_string("/proc/meminfo") {
        let get = |k: &str| -> Option<u64> {
            mem.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok())
        };
        if let (Some(t), Some(a)) = (get("MemTotal:"), get("MemAvailable:")) {
            if t > 0 {
                s.mem_percent = ((1.0 - a as f64 / t as f64) * 1000.0).round() / 10.0;
            }
        }
    }
    if let Some(up) = read_first_line("/proc/uptime") {
        s.uptime_sec = up.split_whitespace().next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) as u64;
    }
    if let Some(df) = run_timeout("df", &["-B1", "/"], 4000) {
        if let Some(last) = df.lines().last() {
            let p: Vec<&str> = last.split_whitespace().collect();
            if p.len() >= 5 {
                s.disk_percent = p[4].trim_end_matches('%').parse().unwrap_or(0.0);
            }
        }
    }
    s
}

#[derive(Default)]
pub struct TmuxInfo {
    pub sessions: Vec<(String, i64, bool)>,
    pub panes: Vec<(String, String, String)>,
}

pub fn collect_tmux() -> TmuxInfo {
    let mut info = TmuxInfo::default();
    if let Some(txt) = run_timeout("tmux", &["list-sessions", "-F", "#{session_name}|#{session_created}|#{session_attached}"], 4000) {
        for l in txt.lines() {
            let p: Vec<&str> = l.split('|').collect();
            if p.len() >= 3 {
                info.sessions.push((p[0].into(), p[1].parse::<i64>().unwrap_or(0) * 1000, p[2] == "1"));
            }
        }
    }
    if let Some(txt) = run_timeout("tmux", &["list-panes", "-a", "-F", "#{session_name}|#{pane_current_command}|#{pane_title}"], 4000) {
        for l in txt.lines() {
            let p: Vec<&str> = l.split('|').collect();
            if p.len() >= 3 {
                info.panes.push((p[0].into(), p[1].into(), redact(p[2]).chars().take(120).collect()));
            }
        }
    }
    info
}

pub fn collect_docker() -> (Vec<String>, Vec<String>) {
    let mut up = vec![];
    let mut down = vec![];
    if let Some(txt) = run_timeout("docker", &["ps", "-a", "--format", "{{.Names}}|{{.Status}}"], 6000) {
        for l in txt.lines() {
            let p: Vec<&str> = l.split('|').collect();
            if p.len() >= 2 {
                if p[1].starts_with("Up") {
                    up.push(p[0].to_string());
                } else {
                    down.push(p[0].to_string());
                }
            }
        }
    }
    (up, down)
}

pub fn collect_git() -> Vec<String> {
    let mut commits = vec![];
    let dir = format!("{}/projects", home());
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.join(".git").exists() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(log) = run_timeout("git", &["-C", &p.to_string_lossy(), "log", "-1", "--format=%h %s"], 3000) {
                let log = redact(log.trim());
                if !log.is_empty() {
                    commits.push(format!("{name}: {log}"));
                }
            }
        }
    }
    commits.truncate(8);
    commits
}

/// Cache OMP: perintah stats/usage berat — hasil bagus terakhir dipakai ulang ≤5 menit.
#[derive(Default)]
pub struct OmpCache {
    pub at: i64,
    pub stats: Option<omp::OmpStats>,
    pub quotas: Vec<Quota>,
}

pub fn collect_omp(cache: &mut OmpCache, force: bool) {
    let now = now_ms();
    if !force && cache.stats.is_some() && now - cache.at < 5 * 60 * 1000 {
        return;
    }
    let stats = run_timeout("omp", &["stats", "--json"], 45000).and_then(|t| omp::parse_stats(&t));
    let quotas = run_timeout("omp", &["usage"], 45000).map(|t| omp::parse_usage(&t)).unwrap_or_default();
    if stats.is_some() {
        cache.stats = stats;
        if !quotas.is_empty() {
            cache.quotas = quotas;
        }
        cache.at = now;
    }
}

/// Hasil probe dasar yang dipakai BERSAMA oleh semua sumber (dihitung sekali).
#[derive(Default)]
pub struct Probes {
    pub now: i64,
    pub procs: Vec<Proc>,
    pub sys: SystemInfo,
    pub tmux: TmuxInfo,
    pub docker_up: Vec<String>,
    pub docker_down: Vec<String>,
    pub commits: Vec<String>,
    pub muse_online: Option<bool>,
}

impl Probes {
    pub fn group(&self, kind: ProcKind) -> Vec<&Proc> {
        self.procs.iter().filter(|p| classify(p) == Some(kind)).collect()
    }
}

/// Keadaan antar-siklus milik kolektor (cache, pembaca CPU).
#[derive(Default)]
pub struct CollectState {
    pub omp_cache: OmpCache,
    pub cpu_state: Option<(u64, u64)>,
    pub heartbeat_dir: String,
}

/// Keluaran satu sumber. Kolom ekstra (kuota/total/saldo) hanya diisi sumber OMP.
#[derive(Default)]
pub struct SourceOut {
    pub agents: Vec<Agent>,
    pub tasks: Vec<Task>,
    pub events: Vec<LogEvent>,
    pub quotas: Option<Vec<Quota>>,
    pub total_usage: Option<Usage>,
    pub deepseek_balance: Option<f64>,
}

pub type SourceFn = fn(&mut CollectState, &Probes) -> SourceOut;

/// REGISTRY SUMBER — tambah konektor baru di sini (satu baris).
pub fn registry() -> Vec<(&'static str, SourceFn)> {
    vec![
        ("omp", src_omp),
        ("cursor", src_cursor),
        ("Muse", src_claude),
        ("coding-agents", src_coding_agents),
        ("mcp", src_mcp),
    ]
}

/// Daftar id sumber yang dikenal (untuk dokumentasi & validasi config).
pub fn known_sources() -> Vec<&'static str> {
    registry().iter().map(|(id, _)| *id).collect()
}

fn source_enabled(id: &str) -> bool {
    if let Ok(only) = std::env::var("HQ_SOURCES") {
        let only: Vec<&str> = only.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if !only.is_empty() {
            return only.contains(&id);
        }
    }
    if let Ok(off) = std::env::var("HQ_DISABLE_SOURCES") {
        let off: Vec<&str> = off.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        return !off.contains(&id);
    }
    true
}

fn usage_for_models(stats: &omp::OmpStats, pred: impl Fn(&omp::ModelUsage) -> bool) -> Usage {
    let mut u = Usage::default();
    for m in &stats.by_model {
        if pred(m) {
            u.requests += m.usage.requests;
            u.ok += m.usage.ok;
            u.failed += m.usage.failed;
            u.input_tokens += m.usage.input_tokens;
            u.output_tokens += m.usage.output_tokens;
            u.cache_tokens += m.usage.cache_tokens;
            u.cash_usd += m.usage.cash_usd;
            u.shadow_usd += m.usage.shadow_usd;
        }
    }
    u
}

// ============================ SUMBER: OMP ============================
fn src_omp(state: &mut CollectState, pr: &Probes) -> SourceOut {
    let now = pr.now;
    collect_omp(&mut state.omp_cache, false);
    let stats = state.omp_cache.stats.clone().unwrap_or_default();
    let mut out = SourceOut::default();

    let sessions = sessions::collect_sessions(now, 24 * 3600 * 1000, 40);
    let recent = |s: &sessions::SessionInfo, mins: i64| now - s.last_ms < mins * 60 * 1000;
    let is_worker = |s: &sessions::SessionInfo| s.agent_type == "subagent" || s.is_subagent;
    let lead_session = sessions.iter().find(|s| !is_worker(s) && recent(s, 120));
    let worker_sessions: Vec<&sessions::SessionInfo> = sessions
        .iter()
        .filter(|s| is_worker(s) && recent(s, 360))
        .collect();

    let omp_procs = pr.group(ProcKind::Omp);
    let omp_recent = stats.last_ts > 0 && now - stats.last_ts < 15 * 60 * 1000;
    let top_model = stats.by_model.first().map(|m| format!("{}/{}", m.provider, m.model));
    let mut lead_usage = stats.main.clone();
    if lead_usage.requests == 0 {
        lead_usage = stats.overall.clone();
    }

    let (lead_activity, lead_status) = if let Some(ls) = lead_session {
        let act = ls
            .last_tools
            .last()
            .map(|t| hq_core::humanize::humanize_tool("OMP", &t.name, &t.intent, &t.target, &ls.project))
            .unwrap_or_else(|| {
                if ls.first_prompt.is_empty() {
                    format!("OMP sedang bekerja di {}", ls.project)
                } else {
                    format!("OMP mengerjakan di {}: {}", ls.project, ls.first_prompt.chars().take(80).collect::<String>())
                }
            });
        let st = if recent(ls, 5) {
            Status::Bekerja
        } else if recent(ls, 30) {
            Status::Berpikir
        } else {
            Status::Santai
        };
        (act, st)
    } else if !omp_procs.is_empty() {
        (
            humanize(&RawActivity {
                agent_name: "OMP".into(),
                raw: omp_procs[0].args.clone(),
                project: None,
                success: None,
                detail: top_model.as_ref().map(|m| format!("model teratas saat ini: {}", hq_core::humanize::friendly_model(m))),
            }),
            if omp_recent { Status::Bekerja } else { Status::Berpikir },
        )
    } else if stats.ok {
        ("OMP sedang santai, menunggu tugas berikutnya".into(), Status::Santai)
    } else {
        ("OMP belum terbaca datanya".into(), Status::Offline)
    };

    out.agents.push(Agent {
        id: "omp".into(),
        name: "OMP".into(),
        emoji: "🧠".into(),
        role: "Mesin coding utama (multi-provider)".into(),
        kind: AgentKind::Utama,
        parent: None,
        status: lead_status,
        activity: lead_activity,
        provider: lead_session
            .map(|s| s.model.split('/').next().unwrap_or("").to_string())
            .filter(|p| !p.is_empty())
            .or_else(|| stats.by_model.first().map(|m| m.provider.clone())),
        model: lead_session
            .map(|s| s.model.clone())
            .filter(|m| !m.is_empty())
            .or_else(|| stats.by_model.first().map(|m| m.model.clone())),
        usage: lead_usage,
        processes: omp_procs.len() as u32,
        cpu_percent: omp_procs.iter().map(|p| p.cpu).sum(),
        active_alerts: 0,
    });

    // Worker per sesi (maks 7 sendiri; sisanya agregat — bawaan Bagian 1).
    if !worker_sessions.is_empty() {
        for ws in worker_sessions.iter().take(7) {
            let wid = format!("omp-w-{}", &ws.id[..ws.id.len().min(8)]);
            let wname = if ws.name.is_empty() {
                format!("Worker · {}", hq_core::humanize::friendly_model(&ws.model))
            } else {
                format!("Worker · {}", ws.name)
            };
            let act = ws
                .last_tools
                .last()
                .map(|t| hq_core::humanize::humanize_tool(&wname, &t.name, &t.intent, &t.target, &ws.project))
                .unwrap_or_else(|| {
                    if ws.first_prompt.is_empty() {
                        format!("{wname} bekerja di {}", ws.project)
                    } else {
                        format!("{wname} di {}: {}", ws.project, ws.first_prompt.chars().take(80).collect::<String>())
                    }
                });
            out.agents.push(Agent {
                id: wid,
                name: wname,
                emoji: "🤖".into(),
                role: format!("Worker OMP di {}", ws.project),
                kind: AgentKind::Worker,
                parent: Some("omp".into()),
                status: if ws.ended {
                    Status::Santai
                } else if recent(ws, 5) {
                    Status::Bekerja
                } else if recent(ws, 30) {
                    Status::Berpikir
                } else {
                    Status::Santai
                },
                activity: act,
                provider: Some(ws.model.split('/').next().unwrap_or("").to_string()),
                model: Some(ws.model.clone()),
                usage: ws.usage.clone(),
                processes: if ws.ended { 0 } else { 1 },
                cpu_percent: 0.0,
                active_alerts: 0,
            });
        }
        if worker_sessions.len() > 7 {
            let rest = &worker_sessions[7..];
            let mut agg = Usage::default();
            for ws in rest {
                agg.requests += ws.usage.requests;
                agg.input_tokens += ws.usage.input_tokens;
                agg.output_tokens += ws.usage.output_tokens;
                agg.cache_tokens += ws.usage.cache_tokens;
                agg.cash_usd += ws.usage.cash_usd;
                agg.shadow_usd += ws.usage.shadow_usd;
            }
            out.agents.push(Agent {
                id: "omp-workers".into(),
                name: format!("OMP Workers (+{})", rest.len()),
                emoji: "🤖".into(),
                role: "Worker OMP lain (dikelompokkan)".into(),
                kind: AgentKind::Worker,
                parent: Some("omp".into()),
                status: Status::Santai,
                activity: format!("{} worker lain tercatat dalam 6 jam terakhir (dikelompokkan agar kantor lega)", rest.len()),
                provider: None,
                model: None,
                usage: agg,
                processes: 0,
                cpu_percent: 0.0,
                active_alerts: 0,
            });
        }
    } else if stats.subagent.requests > 0 || stats.ok {
        let worker_procs = omp_procs.len().saturating_sub(1);
        out.agents.push(Agent {
            id: "omp-workers".into(),
            name: "OMP Workers".into(),
            emoji: "🤖".into(),
            role: "Subagent yang dipanggil OMP Lead".into(),
            kind: AgentKind::Worker,
            parent: Some("omp".into()),
            status: if worker_procs > 0 || omp_recent { Status::Bekerja } else { Status::Santai },
            activity: format!(
                "Tim worker OMP telah menangani {} request ({} token) sebagai subagent",
                stats.subagent.requests, stats.subagent.input_tokens + stats.subagent.output_tokens
            ),
            provider: None,
            model: None,
            usage: stats.subagent.clone(),
            processes: worker_procs as u32,
            cpu_percent: 0.0,
            active_alerts: 0,
        });
    }

    // Tugas & log manusiawi dari sesi.
    let agent_id_of = |s: &sessions::SessionInfo| -> String {
        if is_worker(s) {
            format!("omp-w-{}", &s.id[..s.id.len().min(8)])
        } else {
            "omp".into()
        }
    };
    for s in sessions.iter().filter(|s| recent(s, 360)).take(8) {
        let counts = &s.tool_counts;
        let has = |ks: &[&str]| ks.iter().any(|k| counts.get(*k).copied().unwrap_or(0) > 0);
        let mut steps = vec![];
        if has(&["read", "grep", "glob"]) {
            steps.push(TaskStep { label: "Meneliti kode & berkas".into(), done: s.ended || has(&["edit", "write", "bash"]) });
        }
        if has(&["edit", "write"]) {
            steps.push(TaskStep { label: "Menulis perubahan".into(), done: s.ended });
        }
        if has(&["bash"]) {
            steps.push(TaskStep { label: "Menjalankan perintah & tes".into(), done: s.ended });
        }
        let title = if s.first_prompt.is_empty() {
            format!("Mengerjakan {}", s.project)
        } else {
            s.first_prompt.chars().take(110).collect::<String>()
        };
        out.tasks.push(Task {
            id: format!("session-{}", &s.id[..s.id.len().min(8)]),
            agent_id: agent_id_of(s),
            title,
            detail: format!(
                "{} · {} · {} panggilan perkakas{}",
                s.project,
                hq_core::humanize::friendly_model(&s.model),
                counts.values().sum::<u32>(),
                if s.errors > 0 { format!(" · {} error tercatat", s.errors) } else { String::new() }
            ),
            phase: if s.ended {
                if s.errors > 3 { TaskPhase::Gagal } else { TaskPhase::Selesai }
            } else if recent(s, 10) {
                TaskPhase::Berjalan
            } else {
                TaskPhase::Selesai
            },
            steps,
            started_at: Some(s.started_ms),
            ended_at: if s.ended { Some(s.last_ms) } else { None },
        });
        for t in s.last_tools.iter().rev().take(3) {
            out.events.push(LogEvent {
                at: s.last_ms,
                agent_id: agent_id_of(s),
                kind: "tool".into(),
                human: hq_core::humanize::humanize_tool(
                    if is_worker(s) { "Worker OMP" } else { "OMP" },
                    &t.name,
                    &t.intent,
                    &t.target,
                    &s.project,
                ),
                raw: Some(format!("{} {}", t.name, t.target)),
            });
        }
    }
    out.events.sort_by(|a, b| b.at.cmp(&a.at));
    out.events.truncate(15);

    // Sesi tmux sebagai tugas berjalan pendamping.
    for (name, created, _attached) in &pr.tmux.sessions {
        let pane = pr.tmux.panes.iter().find(|(s, _, _)| s == name);
        let detail = pane.map(|(_, c, t)| format!("{c} — {t}")).unwrap_or_else(|| "sesi terminal aktif".into());
        out.tasks.push(Task {
            id: format!("tmux-{name}"),
            agent_id: "omp".into(),
            title: format!("Sesi kerja \"{name}\" sedang berjalan"),
            detail,
            phase: TaskPhase::Berjalan,
            steps: vec![],
            started_at: Some(*created),
            ended_at: None,
        });
    }
    for c in pr.commits.iter().take(4) {
        out.events.push(LogEvent {
            at: now,
            agent_id: "omp".into(),
            kind: "git".into(),
            human: format!("Hasil kerja terbaru: {c}"),
            raw: Some(c.clone()),
        });
    }

    let deepseek_spent: f64 = stats.by_model.iter().filter(|m| m.provider == "deepseek").map(|m| m.usage.cash_usd).sum();
    out.quotas = Some(state.omp_cache.quotas.clone());
    out.total_usage = Some(usage_for_models(&stats, |_| true));
    out.deepseek_balance = if stats.ok { Some((5.0 - deepseek_spent).max(0.0)) } else { None };
    out
}

// ============================ SUMBER: Cursor ============================
fn src_cursor(_state: &mut CollectState, pr: &Probes) -> SourceOut {
    let mut out = SourceOut::default();
    let cur = pr.group(ProcKind::Cursor);
    let workers = cur.iter().filter(|p| p.args.contains("cursor-agent")).count();
    out.agents.push(Agent {
        id: "cursor".into(),
        name: "Cursor".into(),
        emoji: "🖱️".into(),
        role: "Editor + agent worker (WSL remote)".into(),
        kind: AgentKind::Utama,
        parent: None,
        status: if cur.is_empty() { Status::Offline } else if workers > 0 { Status::Bekerja } else { Status::Santai },
        activity: if cur.is_empty() {
            "Cursor tidak jalan".into()
        } else {
            format!("Cursor aktif: server nyala, {workers} agent worker, {} proses", cur.len())
        },
        provider: None,
        model: None,
        usage: Usage::default(),
        processes: cur.len() as u32,
        cpu_percent: cur.iter().map(|p| p.cpu).sum(),
        active_alerts: 0,
    });
    out
}

// ============================ SUMBER: Muse (heartbeat + Tailscale) ============================
fn src_claude(state: &mut CollectState, pr: &Probes) -> SourceOut {
    let mut out = SourceOut::default();
    let now = pr.now;
    let mut status = Status::Offline;
    let mut activity = "Muse belum melapor".to_string();
    let mut processes = 0u32;
    if let Ok(rd) = std::fs::read_dir(&state.heartbeat_dir) {
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "json").unwrap_or(false) {
                if let Ok(txt) = std::fs::read_to_string(e.path()) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                        let at = v.get("at").and_then(|x| x.as_i64()).unwrap_or(0);
                        let task = v.get("task").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        if now - at < 10 * 60 * 1000 {
                            status = Status::Bekerja;
                            activity = redact(&task);
                            processes = 1;
                        }
                    }
                }
            }
        }
    }
    if status == Status::Offline {
        if pr.muse_online == Some(true) {
            status = Status::Santai;
            activity = "Muse terhubung lewat Tailscale, belum ada laporan tugas baru".into();
        } else if pr.muse_online == Some(false) {
            activity = "Muse tidak terhubung (peer Tailscale offline)".into();
        }
    }
    out.agents.push(Agent {
        id: "Muse".into(),
        name: "Muse".into(),
        emoji: "✨".into(),
        role: "Asisten utama Rizal (VM via Tailscale)".into(),
        kind: AgentKind::Utama,
        parent: None,
        status,
        activity,
        provider: None,
        model: None,
        usage: Usage::default(),
        processes,
        cpu_percent: 0.0,
        active_alerts: 0,
    });
    out
}

// ============================ SUMBER: agent coding lain ============================
fn src_coding_agents(_state: &mut CollectState, pr: &Probes) -> SourceOut {
    let mut out = SourceOut::default();
    for (kind, id, name, emoji, role) in [
        (ProcKind::ClaudeCode, "claude-code", "Claude Code", "⌨️", "Agent coding terminal"),
        (ProcKind::Codex, "codex", "Codex", "🧩", "Agent coding"),
        (ProcKind::Devin, "devin", "Devin", "🛠️", "Agent SWE otonom"),
    ] {
        let g = pr.group(kind);
        if !g.is_empty() {
            out.agents.push(Agent {
                id: id.into(),
                name: name.into(),
                emoji: emoji.into(),
                role: role.into(),
                kind: AgentKind::Utama,
                parent: None,
                status: Status::Bekerja,
                activity: format!("{name} aktif dengan {} proses", g.len()),
                provider: None,
                model: None,
                usage: Usage::default(),
                processes: g.len() as u32,
                cpu_percent: g.iter().map(|p| p.cpu).sum(),
                active_alerts: 0,
            });
        }
    }
    out
}

// ============================ SUMBER: MCP ============================
fn src_mcp(_state: &mut CollectState, pr: &Probes) -> SourceOut {
    let mut out = SourceOut::default();
    let mcps = pr.group(ProcKind::Mcp);
    if !mcps.is_empty() {
        out.agents.push(Agent {
            id: "mcp".into(),
            name: "MCP Servers".into(),
            emoji: "🔌".into(),
            role: "Pelayan perkakas untuk agent lain".into(),
            kind: AgentKind::Mcp,
            parent: None,
            status: Status::Bekerja,
            activity: format!("{} server MCP aktif melayani perkakas", mcps.len()),
            provider: None,
            model: None,
            usage: Usage::default(),
            processes: mcps.len() as u32,
            cpu_percent: mcps.iter().map(|p| p.cpu).sum(),
            active_alerts: 0,
        });
    }
    out
}

/// Bangun snapshot lengkap: probe dasar sekali, lalu jalankan semua sumber aktif.
pub fn collect_snapshot(state: &mut CollectState) -> Snapshot {
    let now = now_ms();
    let mut cpu = state.cpu_state;
    let sys = collect_system(&mut cpu);
    state.cpu_state = cpu;
    let (docker_up, docker_down) = collect_docker();
    let muse_online = run_timeout("tailscale", &["status", "--json"], 5000)
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|j| {
            j.get("Peer")?.as_object()?.values().find(|p| {
                p.get("HostName").and_then(|h| h.as_str()).map(|h| h.to_lowercase().contains("muse")).unwrap_or(false)
            }).and_then(|p| p.get("Online").and_then(|o| o.as_bool()))
        });
    let pr = Probes {
        now,
        procs: collect_processes(),
        sys,
        tmux: collect_tmux(),
        docker_up,
        docker_down,
        commits: collect_git(),
        muse_online,
    };

    let mut snap = Snapshot {
        version: 2,
        generated_at: now,
        ..Default::default()
    };
    for (id, f) in registry() {
        if !source_enabled(id) {
            continue;
        }
        let o = f(state, &pr);
        snap.agents.extend(o.agents);
        snap.tasks.extend(o.tasks);
        snap.events.extend(o.events);
        if let Some(q) = o.quotas {
            snap.quotas = q;
        }
        if let Some(u) = o.total_usage {
            snap.total_usage = u;
        }
        if o.deepseek_balance.is_some() {
            snap.deepseek_balance_est = o.deepseek_balance;
        }
    }
    // Fallback terakhir (aturan Rizal 2 Nov 2026): AKTIF bila ada kuota
    // langganan yang jebol (>=100%) — pekerjaan OMP dialihkan ke DeepSeek tunai.
    let jebol: Vec<String> = snap
        .quotas
        .iter()
        .filter(|q| q.percent.map(|p| p >= 100.0).unwrap_or(false))
        .map(|q| format!("{} {}", q.provider, q.label))
        .collect();
    snap.fallback = FallbackInfo {
        label: "OpenCode Go · DeepSeek V4.1 Flash".into(),
        model: "opencode-go/deepseek-v4.1-flash".into(),
        active: !jebol.is_empty(),
        balance_usd: None,
        reserve_label: "DeepSeek langsung".into(),
        reserve_balance_usd: snap.deepseek_balance_est,
        note: if jebol.is_empty() {
            "Siaga — jalur langganan utama masih tersedia. Bila tumbang: OpenCode Go dulu (region Global aktif, flash + pro terbukti), DeepSeek langsung hanya cadangan terakhir.".into()
        } else {
            format!(
                "AKTIF — kuota jebol: {}. Pekerjaan OMP dialihkan ke OpenCode Go (deepseek-v4.1-flash; kelas berat deepseek-v4-pro; pendukung glm-5.3-flash). Cadangan terakhir DeepSeek langsung — berhenti & lapor bila saldo < $1.",
                jebol.join(", ")
            )
        },
    };
    // Urutan tampil seragam: utama dulu (sesuai urutan registry), lalu worker, lalu mcp.
    snap.env = EnvInfo {
        hostname: pr.sys.hostname.clone(),
        cpu_percent: pr.sys.cpu_percent,
        mem_percent: pr.sys.mem_percent,
        disk_percent: pr.sys.disk_percent,
        uptime_sec: pr.sys.uptime_sec,
        process_total: pr.procs.len() as u32,
        docker_up: pr.docker_up.clone(),
        docker_down: pr.docker_down.clone(),
        recent_commits: pr.commits.clone(),
        tailscale_muse_online: pr.muse_online,
    };
    snap
}

/// Peta kehadiran agent untuk hq-alert.
pub fn agent_presence(procs: &[Proc]) -> HashMap<&'static str, bool> {
    let mut m = HashMap::new();
    let has = |k: ProcKind| procs.iter().any(|p| classify(p) == Some(k));
    m.insert("omp", has(ProcKind::Omp));
    m.insert("cursor", has(ProcKind::Cursor));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_punya_lima_sumber() {
        let ids = known_sources();
        assert_eq!(ids, vec!["omp", "cursor", "Muse", "coding-agents", "mcp"]);
    }
}
