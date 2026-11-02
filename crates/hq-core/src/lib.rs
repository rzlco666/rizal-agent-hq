//! hq-core — model seragam Rizal Agent HQ.
//! Semua panel & API membaca model di sini; tidak ada yang menghitung sendiri.

pub mod humanize;
pub mod redact;
pub mod timefmt;

use serde::{Deserialize, Serialize};

/// Enam status seragam untuk SEMUA agent (PRD §2.1). Tidak boleh ada status lain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Bekerja,
    Berpikir,
    MenungguRizal,
    Santai,
    Offline,
    Error,
}

impl Status {
    pub fn label(&self) -> &'static str {
        match self {
            Status::Bekerja => "Bekerja",
            Status::Berpikir => "Berpikir",
            Status::MenungguRizal => "Menunggu Rizal",
            Status::Santai => "Santai",
            Status::Offline => "Offline",
            Status::Error => "Error",
        }
    }
}

/// Jenis penghuni kantor: utama tampil sebagai manusia, worker/mcp sebagai robot (PRD 3D-C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentKind {
    Utama,
    Worker,
    Mcp,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub requests: u64,
    pub ok: u64,
    pub failed: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    /// Biaya uang nyata (DeepSeek API + model berbayar gateway).
    pub cash_usd: f64,
    /// Biaya setara-API provider langganan (bukan tagihan baru).
    pub shadow_usd: f64,
}

impl Usage {
    pub fn total_cost(&self) -> f64 {
        self.cash_usd + self.shadow_usd
    }
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub emoji: String,
    pub role: String,
    pub kind: AgentKind,
    /// Id agent induk (untuk worker/mcp), None untuk agent utama.
    pub parent: Option<String>,
    pub status: Status,
    /// SATU kalimat tugas manusiawi saat ini (hasil Mesin Humanisasi).
    pub activity: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub usage: Usage,
    pub processes: u32,
    pub cpu_percent: f64,
    /// Jumlah alert aktif milik agent ini.
    pub active_alerts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskPhase {
    Antre,
    Berjalan,
    MenungguRizal,
    Selesai,
    Gagal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub agent_id: String,
    /// Judul manusiawi, mis. "Memperbaiki bug decode MySQL di strut-stack-sql".
    pub title: String,
    pub detail: String,
    pub phase: TaskPhase,
    pub steps: Vec<TaskStep>,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub label: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub at: i64,
    pub agent_id: String,
    pub kind: String,
    /// Kalimat manusiawi (bawaan tampil).
    pub human: String,
    /// Teks mentah asal (di balik toggle "lihat aslinya"), sudah disensor.
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlertLevel {
    Peringatan,
    PerluAction,
    Kritis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlertState {
    Aktif,
    Ditangani,
    Ditunda,
    PulihSendiri,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub agent_id: String,
    pub level: AlertLevel,
    /// Apa yang terjadi, kalimat manusiawi.
    pub title: String,
    /// Persisnya apa yang dibutuhkan dari Rizal.
    pub needs: String,
    pub since: i64,
    pub state: AlertState,
    pub snoozed_until: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Quota {
    pub provider: String,
    pub label: String,
    pub percent: Option<f64>,
    pub extra: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvInfo {
    pub hostname: String,
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub disk_percent: f64,
    pub uptime_sec: u64,
    pub process_total: u32,
    pub docker_up: Vec<String>,
    pub docker_down: Vec<String>,
    pub recent_commits: Vec<String>,
    pub tailscale_muse_online: Option<bool>,
}

/// Rantai fallback pekerjaan OMP (aturan Rizal 2 Nov 2026, diperbarui
/// 16:07 WIB): fallback utama = OpenCode Go (langganan $10/bln); cadangan
/// terakhir = DeepSeek platform langsung (saldo dijaga, berhenti < $1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FallbackInfo {
    pub label: String,
    pub model: String,
    pub active: bool,
    pub balance_usd: Option<f64>,
    pub reserve_label: String,
    pub reserve_balance_usd: Option<f64>,
    pub note: String,
}

/// Snapshot lengkap yang dilayani API & dibaca semua panel.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub generated_at: i64,
    pub agents: Vec<Agent>,
    pub tasks: Vec<Task>,
    pub events: Vec<LogEvent>,
    pub alerts: Vec<Alert>,
    pub quotas: Vec<Quota>,
    pub total_usage: Usage,
    pub deepseek_balance_est: Option<f64>,
    pub fallback: FallbackInfo,
    pub env: EnvInfo,
}
