# 02 — Runtime Adapters

Dokumen ini menetapkan kontrak adapter bukti runtime, identitas, matriks sumber
per runtime, aturan volume event, dan harness paritas. Kontrak crate, tipe inti,
dan katalog event mengikuti [`01-architecture.md`](01-architecture.md) §2, §3,
dan §9; keputusan bukti per runtime mengikuti [`00-design.md`](00-design.md) §10.
Dokumen ini tidak mengulang alasan desain; ia hanya menetapkan bentuk yang
dibangun.

Adapter runtime adalah connector native Tier 1 `local` (00 §11.3). Satu
`LocalConnector` menjalankan semua adapter, menggabungkan fakta per
`runtime_key`, lalu memancarkan `EventDraft` lewat jalur connector yang sama
dengan connector lain. Adapter tidak menulis ke store dan tidak memanggil API.

## 1. Kontrak adapter

### 1.1 Lokasi

| Path | Isi |
|---|---|
| `crates/hq-connectors/src/runtime/mod.rs` | trait `RuntimeAdapter`, tipe fakta, `RuntimeKey`, `Cadence`, `ControlSupport`, `AdapterError` |
| `crates/hq-connectors/src/runtime/cache.rs` | `AdapterCache` (cursor, fingerprint, cache hasil probe mahal) |
| `crates/hq-connectors/src/runtime/proc.rs` | probe `ps` bersama, pemetaan proses → keluarga runtime |
| `crates/hq-connectors/src/runtime/<runtime>.rs` | satu adapter per runtime |
| `crates/hq-connectors/src/identity.rs` | `RuntimeKey` → `Subject`, tabel alias v2 |
| `crates/hq-connectors/src/native/local.rs` | `LocalConnector: Connector`, orkestrasi + normalisasi |
| `crates/hq-connectors/tests/runtime_*.rs` | tes kontrak dan tes per adapter |

`hq-connectors` tidak boleh bergantung pada `hq-store` atau `hq-api`, dan
tidak menulis database apa pun. Pengecualian S09 (lihat 01 §1): `rusqlite`
dipakai hanya untuk pembacaan `~/.omp/stats.db` dengan `mode=ro`,
`query_only=ON`, dan `busy_timeout=2000`. Adapter tetap mengembalikan fakta
atau `EventDraft`, bukan baris DB mentah; `LocalConnector` tetap menjadi satu-
satunya tempat fakta menjadi event untuk umpan balik ke projector.

### 1.2 Signature trait

```rust
// crates/hq-connectors/src/runtime/mod.rs
use async_trait::async_trait;
use hq_core::{
    AgentId, AgentKind, Attribution, EvidenceLevel, Ms, Sentence, SessionId, Status, Usage,
};
use serde::{Deserialize, Serialize};
use crate::secret::SecretProvider;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeId(pub &'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeKey(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostKind {
    Wsl,
    Linux,
    Windows,
    Remote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cadence {
    pub presence_ms: u32,
    pub session_ms: u32,
    pub usage_ms: u32,
    pub quota_ms: u32,
    pub seen_ms: u32,
    pub disappeared_after_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlChannel {
    Cli(&'static str),
    Tmux,
    Signal(&'static str),
    CommandQueue,
    McpTool,
    HttpApi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlCapability {
    Prompt,
    Pause,
    Resume,
    Stop,
    OpenWorkspace,
    ToolCall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSupport {
    pub channels: &'static [ControlChannel],
    pub capabilities: &'static [ControlCapability],
    pub note: &'static str,
}

#[derive(Clone)]
pub struct ObserveCtx {
    pub now_ms: Ms,
    pub deadline_ms: Ms,
    pub home: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub host: HostKind,
    pub cache: AdapterCache,
    pub secrets: Arc<dyn SecretProvider>,
}

#[derive(Clone)]
pub struct RefreshCtx {
    pub now_ms: Ms,
    pub home: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub host: HostKind,
    pub cache: AdapterCache,
    pub secrets: Arc<dyn SecretProvider>,
}

#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    fn id(&self) -> RuntimeId;
    fn display_name(&self) -> &'static str;
    fn evidence_level(&self) -> EvidenceLevel;
    fn cadence(&self) -> Cadence;
    fn control_support(&self) -> ControlSupport;
    fn observe_timeout_ms(&self) -> u32 {
        4_000
    }
    fn refresh_timeout_ms(&self) -> u32 {
        45_000
    }
    async fn refresh(&self, _ctx: &RefreshCtx) -> Result<(), AdapterError> {
        Ok(())
    }
    async fn observe(&self, ctx: &ObserveCtx) -> Result<Observation, AdapterError>;
}
```

`observe` hanya membaca proses, file, SQLite, dan cache. Perintah mahal
(`omp stats --json`, `omp usage`, handshake MCP, HTTP Devin) berjalan di
`refresh`, yang dijadwalkan `LocalConnector` di latar belakang dan tidak
menahan snapshot. `observe` wajib selesai dalam `observe_timeout_ms`; `refresh`
wajib selesai dalam `refresh_timeout_ms`.

### 1.3 Tipe fakta

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Observation {
    pub agents: Vec<AgentFacts>,
    pub sessions: Vec<SessionFacts>,
    pub usage: Vec<UsageFacts>,
    pub quota: Vec<QuotaFacts>,
    pub logs: Vec<LogFacts>,
    pub notes: Vec<EvidenceNote>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentFacts {
    pub runtime_key: RuntimeKey,
    pub subject: AgentId,
    pub parent: Option<AgentId>,
    pub kind: AgentKind,
    pub display_name: String,
    pub status: Status,
    pub status_reason: StatusReason,
    pub activity: Sentence,
    pub processes: u32,
    pub cpu_percent: f32,
    pub last_seen_ms: Ms,
    pub sources: Vec<EvidenceSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusReason {
    ProcessActive,
    TranscriptActive,
    ExplicitWaiting,
    ExplicitError,
    HeartbeatFresh,
    ProcessIdle,
    NoEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceSource {
    Process,
    Transcript,
    Usage,
    Quota,
    Api,
    Heartbeat,
    Config,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceNote {
    pub source: EvidenceSource,
    pub level: EvidenceLevel,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionFacts {
    pub subject: SessionId,
    pub runtime_key: RuntimeKey,
    pub agent: AgentId,
    pub cwd: Option<String>,
    pub started_ms: Ms,
    pub last_ms: Ms,
    pub ended: bool,
    pub outcome: Option<String>,
    pub title: Option<Sentence>,
    pub model: Option<String>,
    pub usage: Usage,
    pub source_cursor: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageFacts {
    pub runtime_key: RuntimeKey,
    pub session: Option<SessionId>,
    pub provider: String,
    pub model: String,
    pub attribution: Attribution,
    pub cumulative: Usage,
    pub observed_at_ms: Ms,
    pub source_cursor: String,
    pub reset: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuotaFacts {
    pub runtime_key: RuntimeKey,
    pub provider: String,
    pub label: String,
    pub percent: Option<f64>,
    pub detail: String,
    pub resets_ms: Option<Ms>,
    pub observed_at_ms: Ms,
    pub source_cursor: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogFacts {
    pub runtime_key: RuntimeKey,
    pub session: Option<SessionId>,
    pub ts_ms: Ms,
    pub level: LogLevel,
    pub sentence: Sentence,
    pub raw: Option<String>,
    pub source_cursor: String,
}
```

`EvidenceNote.message` sudah melewati `hq_core::redact`, maksimal 200 karakter,
dan tidak boleh berisi path berisi token, argv, header, atau isi prompt.
`SessionFacts.subject` memakai bentuk kanonik 01 §2; `runtime_key` tetap milik
instance agent, bukan subjek sesi.

### 1.4 Error dan cache

```rust
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("unavailable: {code}")]
    Unavailable { code: &'static str },
    #[error("timeout after {0} ms")]
    Timeout(u32),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {path}: {reason}")]
    Parse { path: String, reason: String },
    #[error("sqlite: {0}")]
    Sqlite(String),
    #[error("http: {status}")]
    Http { status: u16 },
    #[error("auth failed")]
    Auth,
    #[error("schema mismatch: {0}")]
    Schema(String),
}

#[derive(Clone, Debug)]
pub struct AdapterCache {
    root: PathBuf,
}

impl AdapterCache {
    pub fn new(root: PathBuf) -> Self;
    pub fn read<T: serde::de::DeserializeOwned>(&self, id: RuntimeId, key: &str) -> Option<T>;
    pub fn write<T: serde::Serialize>(
        &self,
        id: RuntimeId,
        key: &str,
        value: &T,
    ) -> Result<(), AdapterError>;
    pub fn last_refresh_ms(&self, id: RuntimeId) -> Option<Ms>;
    pub fn mark_refreshed(&self, id: RuntimeId, at_ms: Ms) -> Result<(), AdapterError>;
    pub fn cursor(&self, id: RuntimeId, name: &str) -> Option<String>;
    pub fn set_cursor(&self, id: RuntimeId, name: &str, value: &str)
        -> Result<(), AdapterError>;
}

// SecretProvider, SecretString, SecretError: lihat 03 §1.3.
// Adapter memakai trait yang sama; tidak ada trait secret kedua.
```

`AdapterCache` menulis `HQ_DATA/adapters/<runtime_id>/<key>.json` dengan
mode 0600, tulis atomik `tmp → fsync → rename`, dan memulihkan file rusak
sebagai `<key>.corrupt-<ts>` lalu memulai kosong. Isi cache: cursor file/DB,
fingerprint terakhir, `last_refresh_ms`, dan hasil parse terakhir. Tidak ada
secret, raw prompt, header, atau token di cache.

### 1.5 Cara `LocalConnector` memanggil adapter

`LocalConnector` mengimplementasikan `Connector` (03 §1) dengan manifest
`id = local`, `tier = native`, `poll: None`, `webhook: None`, `actions: {}`.
`poll` melakukan:

1. `schedule_refresh`: untuk setiap adapter yang `now_ms - last_refresh_ms >=
   cadence.usage_ms` dan belum punya refresh berjalan, spawn satu task
   `tokio::spawn` dengan `refresh_timeout_ms`; task menulis cache lalu
   `mark_refreshed`. `AtomicBool` per adapter mencegah dua refresh paralel.
2. Membuat satu `ObserveCtx` clone-able dengan `deadline_ms` absolut =
   `now_ms + min(HQ_INTERVAL * 1000 - 500, 5_000)`.
3. Menjalankan semua `observe` secara konkuren dalam `tokio::task::JoinSet`,
   masing-masing dibungkus `tokio::time::timeout(observe_timeout_ms)`.
4. Mengumpulkan `(RuntimeId, Result<Observation, AdapterError>)`. Satu adapter
   gagal tidak membatalkan yang lain; task yang panic ditangkap sebagai
   `JoinError` dan dicatat sebagai kegagalan adapter itu.
5. `normalize` menggabungkan fakta per `runtime_key` (02 §2), menghitung
   fingerprint, memeriksa dedupe key, lalu mengembalikan `Vec<EventDraft>`.
6. Jika semua adapter gagal, `poll` mengembalikan
   `Err(ConnectorError::Degraded("adapter_all_failed".into()))`; supervisor
   menaikkan backoff. Jika
   sebagian gagal, `poll` mengembalikan `Ok` dan health connector menjadi
   `degraded` dengan `last_error.code = "adapter_partial_failure"`.

Skeleton orkestrasi:

```rust
pub struct LocalConnector {
    manifest: Manifest,
    adapters: Vec<AdapterEntry>,
    cache: AdapterCache,
}

pub struct AdapterEntry {
    pub adapter: Arc<dyn RuntimeAdapter>,
    pub refreshing: Arc<std::sync::atomic::AtomicBool>,
}

#[async_trait]
impl Connector for LocalConnector {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn poll(&self, ctx: &ConnectorCtx<'_>) -> Result<Vec<EventDraft>, ConnectorError> {
        self.schedule_refresh(ctx);

        let octx = ObserveCtx {
            now_ms: ctx.now_ms,
            deadline_ms: ctx.deadline_ms,
            home: ctx.home.clone(),
            config_dir: ctx.config_dir.clone(),
            data_dir: ctx.data_dir.clone(),
            host: ctx.host,
            cache: self.cache.clone(),
            secrets: Arc::clone(&ctx.secrets),
        };

        let mut set = tokio::task::JoinSet::new();
        for entry in &self.adapters {
            let adapter = Arc::clone(&entry.adapter);
            let octx = octx.clone();
            set.spawn(async move {
                let timeout_ms = adapter.observe_timeout_ms();
                match tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms as u64),
                    adapter.observe(&octx),
                )
                .await
                {
                    Ok(Ok(obs)) => (adapter.id(), Ok(obs)),
                    Ok(Err(err)) => (adapter.id(), Err(err)),
                    Err(_) => (adapter.id(), Err(AdapterError::Timeout(timeout_ms))),
                }
            });
        }

        let mut results = Vec::new();
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(result) => results.push(result),
                Err(_) => results.push((
                    RuntimeId("unknown"),
                    Err(AdapterError::Unavailable { code: "adapter_panic" }),
                )),
            }
        }

        self.normalize(results, ctx.now_ms)
    }

    async fn handle_webhook(
        &self,
        _req: WebhookRequest,
    ) -> Result<Vec<EventDraft>, ConnectorError> {
        Err(ConnectorError::Unsupported("local has no webhook".into()))
    }

    async fn action(
        &self,
        _action: &str,
        _input: serde_json::Value,
    ) -> Result<ActionOutcome, ConnectorError> {
        Err(ConnectorError::Unsupported("control lives in hq-control".into()))
    }

    async fn health(&self) -> Health {
        self.aggregate_health()
    }
}
```

`normalize` tidak boleh memancarkan event untuk fakta yang tidak berubah;
aturan lengkap ada di 02 §4.

## 2. Identitas dan penggabungan

### 2.1 Skema `runtime_key`

`runtime_key` adalah string stabil yang mengikat proses, transcript, dan usage
ke satu instance. Grammar:

```text
runtime_key := family
             | family ":" kind ":" id
             | "heartbeat:" slug
family      := [a-z0-9][a-z0-9-]*
kind        := "session" | "workspace" | "worker" | "server"
id          := [A-Za-z0-9._:-]+
slug        := [a-z0-9][a-z0-9-]{1,47}
```

| Runtime | `runtime_key` instance | `Subject` agent | `Subject` sesi/instance |
|---|---|---|---|
| OMP utama | `omp` | `agent:omp` | — |
| OMP sesi | `omp:session:<session_id>` | `agent:omp` | `agent:omp/session/<session_id>` |
| OMP worker | `omp:worker:<session_id>` | `agent:omp/worker/<session_id>` (parent `agent:omp`) | `agent:omp/worker/<session_id>` |
| OMP worker agregat (paritas) | `omp:worker:agg` | `agent:omp/worker/agg` (parent `agent:omp`) | `agent:omp/worker/agg` |
| Cursor workspace | `cursor:workspace:<sha1(abs_path)[..12]>` | `agent:cursor` | `agent:cursor/workspace/<hash>` |
| Cursor transcript | `cursor:workspace:<hash>` | `agent:cursor` | `agent:cursor/session/<uuid>` |
| Claude Code | `claude-code` | `agent:claude-code` | — |
| Claude Code sesi | `claude-code:session:<uuid>` | `agent:claude-code` | `agent:claude-code/session/<uuid>` |
| Codex | `codex` | `agent:codex` | — |
| Codex sesi | `codex:session:<uuid>` | `agent:codex` | `agent:codex/session/<uuid>` |
| Gemini CLI | `gemini-cli` | `agent:gemini-cli` | — |
| Gemini CLI sesi | `gemini-cli:session:<uuid>` | `agent:gemini-cli` | `agent:gemini-cli/session/<uuid>` |
| OpenCode | `opencode` | `agent:opencode` | — |
| OpenCode sesi | `opencode:session:<ses_id>` | `agent:opencode` | `agent:opencode/session/<ses_id>` |
| Copilot | `copilot` | `agent:copilot` | — |
| Muse | `muse` | `agent:muse` | — |
| Devin | `devin` | `agent:devin` | — |
| Devin sesi | `devin:session:<id>` | `agent:devin` | `agent:devin/session/<id>` |
| MCP agregat | `mcp` | `agent:mcp` | — |
| MCP server | `mcp:server:<slug>` | `agent:mcp/<slug>` (parent `agent:mcp`) | `agent:mcp/<slug>` |
| Heartbeat eksternal | `heartbeat:<slug>` | `agent:<slug>` | — |

Aturan:

- Satu `runtime_key` = satu agent subject. Dua sumber tidak boleh membuat dua
  agent bila `runtime_key` sama (00 §9.2.1).
- `agent:omp` dan `agent:cursor` selalu ada di registry; subjek utama lain ada
  bila runtime terkonfigurasi atau pernah punya bukti.
- `agent:mcp` adalah parent agregat; setiap server adalah child
  `agent:mcp/<slug>`. Ini mempertahankan padanan v2 `mcp` tanpa menggabungkan
  server yang berbeda.
- `agent:omp/worker/agg` adalah subjek sintetis untuk paritas v2. Ia tidak
  ditampilkan di Today secara default; dipakai normalizer paritas dan riwayat.
- Slug heartbeat: `[a-z0-9][a-z0-9-]{1,47}`; nama tampilan disimpan terpisah.
- Runtime yang memakai key sesi/worker (OMP, Claude Code, Codex, Gemini CLI,
  OpenCode, Devin, MCP) menggabungkan fakta anak ke kartu parent untuk status
  dan activity: parent memakai proses + sesi/worker terbaru. Anak tetap punya
  subjek sendiri; ia tidak menggandakan kartu parent. Cursor tidak memakai
  key sesi terpisah: transcript-nya selalu masuk ke key workspace.

### 2.2 Alias v2 → v3

Baris `identity_aliases` yang diisi migrasi S25:

| Alias v2 | Canonical v3 | Catatan |
|---|---|---|
| `omp` | `agent:omp` | runtime utama |
| `cursor` | `agent:cursor` | runtime utama |
| `Muse` | `agent:muse` | case-sensitive di v2 |
| `mcp` | `agent:mcp` | agregat; child per server |
| `omp-workers-agg` | `agent:omp/worker/agg` | agregat worker |
| `omp-workers` | `agent:omp/worker/agg` | id aktual v2 |
| `claude-code` | `agent:claude-code` | hanya ada bila proses v2 terlihat |
| `codex` | `agent:codex` | hanya ada bila proses v2 terlihat |
| `devin` | `agent:devin` | hanya ada bila proses v2 terlihat |

Alias dinamis v2 (id terpotong 8 karakter) tidak masuk tabel; normalizer
paritas mencocokkan prefiks secara unik:

| Prefiks v2 | Template v3 | Aturan |
|---|---|---|
| `omp-s-` | `agent:omp/session/<full_id>` | cocokkan prefiks 8 karakter; ambigu = unmatched |
| `omp-w-` | `agent:omp/worker/<full_id>` | cocokkan prefiks 8 karakter; ambigu = unmatched |

Nama tampilan tidak pernah menjadi identitas. Perubahan nama tidak mengubah
subject dan tidak menghasilkan `agent.disappeared`.

### 2.3 Aturan merge

`LocalConnector::normalize` menerima semua `Observation`, mengelompokkan
`AgentFacts`/`SessionFacts`/`UsageFacts` berdasarkan `runtime_key`, lalu:

1. **Subject.** Ambil `subject` dari tabel §2.1. Jika fakta membawa subject
   berbeda untuk key yang sama, fakta itu dibuang dan dicatat sebagai
   `EvidenceNote { code: "subject_conflict" }`.
2. **Display name.** Registry runtime menang. Heartbeat memakai nama dari
   payload; jika tidak ada, slug. Transcript dan proses tidak boleh mengubah
   nama.
3. **Status.** Pilih dengan urutan prioritas berikut; nilai pertama yang
   terpenuhi menang:
   1. `Error` — ada sinyal error eksplisit dari transcript/API dalam 5 menit.
   2. `MenungguRizal` — heartbeat/transcript menyatakan menunggu dalam 2 jam.
   3. `Bekerja` — proses hidup dan (`cpu_percent >= 1.0` atau transcript
      terakhir ≤ 5 menit atau delta usage ≤ 5 menit).
   4. `Berpikir` — proses hidup dan transcript terakhir ≤ 30 menit tanpa
      prompt/tool selesai ≤ 5 menit.
   5. `Santai` — proses hidup tanpa aktivitas ≤ 30 menit, atau heartbeat segar
      tanpa task.
   6. `Offline` — tidak ada proses, transcript lebih tua dari
      `cadence.disappeared_after_ms`, dan heartbeat kedaluwarsa.
   `Error` dan `MenungguRizal` tidak boleh diturunkan oleh sumber lain.
4. **Activity.** Pilih `Sentence` dari sumber dengan `EvidenceLevel` tertinggi
   dan `last_seen_ms` terbaru. Dua fakta dengan level sama: transcript menang
   atas proses; proses menang atas config.
5. **Processes.** `max`, bukan jumlah; satu probe `ps` adalah sumber tunggal.
6. **CPU.** Jumlahkan hanya proses yang dipetakan ke `runtime_key` yang sama;
   jangan jumlahkan proses anak dua kali.
7. **Usage.** Untuk setiap `(runtime_key, session, provider, model)` pilih satu
   sumber kumulatif: API/DB terstruktur > transcript `model_usage` > estimasi.
   Jangan menjumlahkan dua sumber untuk kumulatif yang sama. Delta dihitung
   sekali oleh `LocalConnector` terhadap baseline cache.
8. **Evidence.** Level agent = level adapter, tetapi diturunkan menjadi L1 bila
   hanya proses, dan `Offline` bila tidak ada sumber. Konfigurasi saja tidak
   pernah menaikkan level.

Status `Offline` tetap mempertahankan `last_seen_ms` dan bukti terakhir. Event
`agent.disappeared` dipancarkan sekali per episode hilang, bukan setiap tick.

### 2.4 Waktu dan event terlambat

- `event_ts_ms` = timestamp sumber (`session.timestamp`, `model_usage.timestamp`,
  `time_updated`, heartbeat `at_ms`, HTTP `updated_at`).
- `ingest_ts_ms` diisi store saat append. Transcript yang ditemukan belakangan
  tetap memakai `event_ts_ms` asli; projection memperbarui bucket waktu yang
  terpengaruh (00 §9.2.2).
- Urutan append tidak boleh bergantung pada urutan file. Dedupe key memakai
  cursor sumber, bukan waktu ingest.
- File yang lebih tua dari 90 hari tetap dibaca untuk cursor, tetapi event
  operasionalnya masuk retensi normal di store.

## 3. Matriks runtime

Tabel ringkas. Level mengikuti 00 §10 dan paragraf verifikasi 2026-11-16.
Catatan rekonsiliasi: tabel 00 §10 menulis OpenCode L2, sedangkan paragraf
verifikasi menetapkan L3 berbasis SQLite; dokumen ini memakai **L3** karena
`session` OpenCode menyimpan token dan biaya terstruktur.

| Runtime | Bukti | Level | Kontrol | Adapter file |
|---|---|---|---|---|
| OMP | proses, session JSONL, `model_usage`, `stats.db`, `omp stats`, `omp usage` | L3 | CLI, tmux, sinyal | `runtime/omp.rs` |
| Cursor + cursor-agent | proses, transcript workspace, state lokal | L2 | buka workspace | `runtime/cursor.rs` |
| Claude Code | proses, transcript `~/.claude` | L3 | CLI, sinyal | `runtime/claude_code.rs` |
| Codex | proses, rollout `~/.codex` | L3 | CLI, sinyal | `runtime/codex.rs` |
| Gemini CLI | proses, sesi lokal | L2 | CLI | `runtime/gemini_cli.rs` |
| OpenCode | proses, `opencode.db` | L3 | CLI | `runtime/opencode.rs` |
| Copilot | proses/CLI, log | L1–L2 | CLI jika ada | `runtime/copilot.rs` |
| Muse | heartbeat, Tailscale | L2 | command queue | `runtime/muse.rs` |
| Devin | API cloud | L2 | API action | `runtime/devin.rs` |
| MCP server | config, proses, handshake `tools/list` | L3 | MCP tool call | `runtime/mcp.rs` |
| Heartbeat generik | POST + last-seen | L1–L2 | command queue | `runtime/heartbeat.rs` |

### 3.1 OMP

| Item | Nilai |
|---|---|
| Sumber terverifikasi | `ps` pola `/omp`, `omp`, `oh-my-pi`, `pi-coding-agent`; `~/.omp/agent/sessions/<project>/*.jsonl`; subagent `.../<session_id>/<name>.jsonl`; `~/.omp/stats.db` tabel `session_rollup`, `messages`, `tool_calls`; perintah `omp stats --json`; `omp usage` |
| Parse presence | `ps -eo pid=,pcpu=,pmem=,etime=,comm=,args=`; filter pola di atas; `cpu_percent` jumlah; `processes` jumlah; argv di-redact |
| Parse session | JSONL baris: `type:"title"` → title; `type:"session"` → `id`, `timestamp`, `cwd`, `parentSession`; `type:"model_change"` → model; `type:"message"` → role, text, `toolCall.name`; `type:"model_usage"` → provider, model, usage, cost; `type:"custom", customType:"session_exit"` → ended + reason |
| Parse usage | `model_usage.usage.{input,output,cacheRead,cacheWrite,totalTokens,reasoningTokens,cost.total}`; split cash/shadow dengan aturan `omp::split_one` (v2); `stats.db` dipakai untuk rollup/backfill; `omp stats --json` untuk `overall`/`byModel`; `omp usage` untuk kuota provider |
| Cadence | presence 6000 ms; session 30000 ms; usage 60000 ms (dari `model_usage`), `omp stats`/`omp usage` refresh 300000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `session.started`, `session.updated`, `session.ended`, `usage.recorded`, `quota.observed`, `log.recorded` |
| Dedupe | `session.started:omp/session/<id>:<timestamp>`; `session.updated:omp/session/<id>:<last_line_offset>`; `session.ended:...:<exit_ts>`; `usage:omp:<session>:<provider>:<model>:<cursor_hash>`; `quota:omp:<provider>:<label>:<percent_x10>:<reset_bucket>`; `log:omp:<session>:<line_offset>` |
| Level | L3 |
| Kontrol | `Cli("omp")` Prompt/Pause/Resume/Stop; `Tmux` send-keys; `Signal("SIGINT")`, `Signal("SIGTERM")` |
| Degradasi | Binary hilang tetapi session/`stats.db` ada → tetap L3 untuk riwayat, status `Offline` dengan catatan `binary_absent`; hanya config (`~/.omp/agent/config.yml`) tanpa session/stats → `Offline`, catatan `config_only`; JSONL rusak/terpotong → offset reset, parse dari awal, tidak ada usage delta ganda |

`session_rollup` query:

```sql
SELECT session_file, requests, started_at, ended_at, total_tokens,
       cost_total, unpriced, models, tool_calls
FROM session_rollup
WHERE ended_at > ?1
ORDER BY ended_at ASC
LIMIT 500;
```

`omp stats --json` memakai field yang sudah diverifikasi parser v2:
`overall.totalRequests`, `overall.successfulRequests`, `overall.failedRequests`,
`overall.totalInputTokens`, `overall.totalOutputTokens`,
`overall.totalCacheReadTokens`, `overall.totalCost`, `overall.errorRate`,
`overall.lastTimestamp`, dan `byModel[].{provider, model, totalCost,
totalRequests, successfulRequests, failedRequests, totalInputTokens,
totalOutputTokens, totalCacheReadTokens}`.
`omp usage` adalah teks; parser memakai regex v2 untuk baris
`42.0% used · resets in 15d17h` dan `12 / 100 requests`. Jika format
berubah, refresh mencatat `parse_usage_failed` dan mempertahankan cache.

### 3.2 Cursor (+ cursor-agent)

| Item | Nilai |
|---|---|
| Sumber terverifikasi | `ps` pola `cursor-server`, `cursor-agent`, `.cursor/`; `~/.cursor/projects/<slug>/agent-transcripts/<uuid>/<uuid>.jsonl`; `~/.cursor/agent-cli-state.json`; `~/.cursor/ide_state.json`; `~/.cursor/cli-config.json`; `~/.cursor/mcp.json` (MCP ditangani adapter MCP) |
| Parse presence | `cursor-agent` dihitung worker; `cursor-server` = editor; proses saja tidak mengklaim task |
| Parse transcript | JSONL per baris: `{role, message:{content:[{type:"text"|"tool_use", ...}]}}`; user query diambil dari teks pertama; tool terakhir dari `tool_use.name`; timestamp dari `<timestamp>...</timestamp>` bila ada, atau mtime file; session id = nama direktori/file UUID |
| Identitas workspace | `agent-cli-state.json` `workerIdsByDisplayName` memberi path absolut + host. Normalisasi path ke slug (`non-alnum → '-'`, lowercase) lalu cocokkan dengan `<slug>`. Jika cocok: `sha1(abs_path)[..12]`; jika tidak (direktori hash): `sha1("cursor-project:"+slug)[..12]` dan catat `workspace_hash_fallback` |
| Cadence | presence 6000 ms; transcript 30000 ms; state 60000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `session.started`, `session.updated`, `session.ended`, `log.recorded` |
| Dedupe | `session.started:cursor/session/<uuid>:<mtime_first>`; `session.updated:...:<file_offset>`; `log:cursor:<session>:<file_offset>`; workspace agent memakai `observed:<runtime_key>:<fingerprint>:<bucket>` |
| Level | L2 |
| Kontrol | `Cli("cursor")` OpenWorkspace; tidak ada API chat |
| Degradasi | Hanya `agent-cli-state.json`/`ide_state.json` → L1, bukan aktivitas; transcript hilang → proses saja L1; process hilang → `Offline` setelah 120 s; file terpotong → offset reset, tidak ada usage (Cursor tidak punya usage terstruktur) |

`cursor-agent` bukan runtime terpisah: ia memperkuat bukti workspace yang sama.
Transcript Cursor tidak memuat token/biaya; karena itu level tetap L2 dan
`usage.recorded` tidak dipancarkan dari Cursor.

### 3.3 Claude Code

| Item | Nilai |
|---|---|
| Sumber | `ps` pola `claude`, `claude-code`, `@anthropic-ai/claude`; transcript `~/.claude/projects/**/*.jsonl` (jalur konvensi Claude Code); config `~/.claude/settings.json` |
| Status mesin ini | Binary `claude` tidak ada. `~/.claude` ada (settings + skills), tetapi tidak ada `projects/` transcript. Konfigurasi saja bukan aktivitas. |
| Parse transcript | JSONL; ambil `type` (`user`/`assistant`/`system`/`summary`), `sessionId`, `uuid`, `timestamp`, `cwd`, `message.role`, `message.content[]`, `message.usage`; tool dari `content[].name`; error dari `type:"system"` dengan level error |
| Parse usage | `message.usage.input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`; akumulasi per session/model; biaya dihitung projector dari tabel harga, bukan adapter |
| Cadence | presence 6000 ms; transcript 30000 ms; usage dari transcript 60000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `session.started`, `session.updated`, `session.ended`, `usage.recorded`, `log.recorded` |
| Dedupe | `session.started:claude-code/session/<uuid>:<first_ts>`; `session.updated:...:<file_offset>`; `usage:claude-code:<session>:<provider>:<model>:<cursor_hash>`; `log:claude-code:<session>:<message_uuid>` |
| Level | L3 bila transcript ada; L1 bila hanya proses; `Offline` bila tidak ada keduanya |
| Kontrol | `Cli("claude")` Prompt/Pause/Resume/Stop; `Signal("SIGINT")`; jika binary tidak ada, `hq-control` mengembalikan `unavailable` |
| Degradasi | Binary hilang + transcript ada → L3 riwayat, status `Offline`; binary ada + transcript hilang → L1; hanya config → `Offline`, catatan `config_only`, tanpa event agent |

Claude Code tidak dijalankan oleh adapter. Adapter tidak pernah menebak task
dari nama proses; task hanya dari transcript.

### 3.4 Codex

| Item | Nilai |
|---|---|
| Sumber | `ps` pola `codex`; rollout `~/.codex/sessions/**/rollout-*.jsonl`; config `~/.codex/config.toml`; hooks `~/.codex/hooks.json` |
| Status mesin ini | Binary `codex` tidak ada; `~/.codex` hanya `config.toml` + `hooks.json`; tidak ada rollout. |
| Parse rollout | JSONL toleran: `type:"session_meta"` → `payload.id`, `payload.timestamp`, `payload.cwd`; `type:"event_msg"` dengan `payload.type:"token_count"` → `payload.info.total_token_usage.{input_tokens,cached_input_tokens,output_tokens,reasoning_output_tokens}`; `type:"event_msg"` lain → log; `type:"response_item"` → pesan/tool; `type:"error"` → status error |
| Cadence | presence 6000 ms; rollout 30000 ms; usage 60000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `session.started`, `session.updated`, `session.ended`, `usage.recorded`, `log.recorded` |
| Dedupe | `session.started:codex/session/<uuid>:<first_ts>`; `session.updated:...:<file_offset>`; `usage:codex:<session>:<provider>:<model>:<cursor_hash>`; `log:codex:<session>:<line_offset>` |
| Level | L3 bila rollout ada; L1 bila hanya proses; `Offline` bila tidak ada |
| Kontrol | `Cli("codex")` Prompt/Pause/Resume/Stop; `Signal("SIGINT")` |
| Degradasi | Rollout hilang → L1; hanya config → `Offline`, catatan `config_only`; format rollout baru yang tidak dikenal → presence + note `schema_unknown`, tidak ada event usage |

Rollout Codex tidak selalu memuat `provider`; adapter memakai `provider =
"openai-codex"` bila tidak ada dan `model` dari `turn_context`/`session_meta`.
Biaya dihitung projector.

### 3.5 Gemini CLI

| Item | Nilai |
|---|---|
| Sumber | `ps` pola `gemini`; sesi `~/.gemini/tmp/**/chats/session-*.json`; config `~/.gemini/settings.json`, `~/.gemini/config/projects/*.json` |
| Status mesin ini | Binary `gemini` tidak ada. `~/.gemini` berisi `antigravity-*`; itu produk berbeda dan **tidak** dibaca adapter Gemini CLI. |
| Parse sesi | JSON: `sessionId`, `startTime`, `lastUpdated`, `messages[]`; pesan punya `type`, `content`, `timestamp`, optional `usageMetadata`; tool dari `toolCalls[].name`; error dari `type:"error"` |
| Cadence | presence 6000 ms; session 30000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `session.started`, `session.updated`, `session.ended`, `log.recorded` |
| Dedupe | `session.started:gemini-cli/session/<uuid>:<startTime>`; `session.updated:...:<lastUpdated>`; `log:gemini-cli:<session>:<message_index>` |
| Level | L2; `usageMetadata` tidak menaikkan level karena format belum stabil |
| Kontrol | `Cli("gemini")` Prompt/Pause/Resume/Stop |
| Degradasi | Sesi hilang → L1 proses; hanya config → `Offline`; `antigravity-*` diabaikan dengan catatan `out_of_scope_store` |

### 3.6 OpenCode

| Item | Nilai |
|---|---|
| Sumber terverifikasi | `ps` pola `opencode`; `~/.local/share/opencode/opencode.db`; log `~/.local/share/opencode/log/opencode.log`; config `~/.config/opencode/opencode.jsonc` |
| Parse DB | SQLite read-only (`mode=ro`, `busy_timeout=250`); query `session` dengan kolom `id,parent_id,directory,title,model,agent,cost,tokens_input,tokens_output,tokens_reasoning,tokens_cache_read,tokens_cache_write,time_created,time_updated,time_archived`; `WHERE time_updated > ?1 AND time_archived IS NULL ORDER BY time_updated ASC LIMIT 500`; pesan terakhir dari `message`/`part` dengan indeks `message_session_time_created_id_idx` |
| Parse usage | `cost` → `cash_usd` bila provider berbayar; token: `tokens_input`, `tokens_output`, `tokens_cache_read`, `tokens_cache_write`; `tokens_reasoning` dijumlahkan ke output; `model` string `provider/model` dipecah |
| Cadence | presence 6000 ms; DB 30000 ms; usage 60000 ms; log 60000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `session.started`, `session.updated`, `session.ended`, `usage.recorded`, `log.recorded` |
| Dedupe | `session.started:opencode/session/<ses_id>:<time_created>`; `session.updated:...:<time_updated>`; `usage:opencode:<session>:<provider>:<model>:<cursor_hash>`; `log:opencode:<session>:<message_id>` |
| Level | L3 |
| Kontrol | `Cli("opencode")` Prompt/Pause/Resume/Stop |
| Degradasi | DB tidak ada → L1 proses; DB terkunci (`SQLITE_BUSY`) → pakai cache, health `degraded`, tidak menggantung; skema tidak cocok → `Error` dengan `schema_mismatch`, tidak ada data palsu; sesi `parent_id` tetap subjek sesi, bukan worker |

`time_updated` OpenCode adalah epoch ms. `time_archived` non-null berarti sesi
diarsipkan: pancarkan `session.ended` sekali bila sebelumnya aktif, lalu
berhenti memperbarui.

### 3.7 Copilot

| Item | Nilai |
|---|---|
| Sumber terverifikasi | `ps` pola `copilot`, `copilot-shim`, `github-copilot`; `~/.copilot/logs/process-*.log`; `~/.copilot/ide`; `~/.copilot/mcp-config.json`; binary `copilot` shim VS Code |
| Parse | Presence dari proses. Log baris `process-*.log` diparse untuk `session`, `request`, `error`; tidak ada token/biaya terstruktur. `gh` hanya bukti lingkungan, bukan aktivitas Copilot |
| Cadence | presence 6000 ms; log 60000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `log.recorded` |
| Dedupe | `log:copilot:<process_id>:<line_offset>`; agent memakai `observed:<runtime_key>:<fingerprint>:<bucket>` |
| Level | L1; L2 hanya bila log memuat session id eksplisit, tetap tanpa usage |
| Kontrol | `Cli("copilot")` Prompt/Stop bila binary ada; catatan: shim VS Code, tidak ada API chat |
| Degradasi | Shim ada tetapi tidak ada proses → `Offline`; hanya config → `Offline`; log kosong → L1, bukan task palsu |

### 3.8 Muse

| Item | Nilai |
|---|---|
| Sumber | heartbeat `HQ_DATA/heartbeats/muse.json` (format v2 `{at, task, status}`) dan `POST /api/v1/heartbeat`; `tailscale status --json`; peer dikonfigurasi `HQ_MUSE_PEER` (default `muse`) |
| Parse | Heartbeat terbaru menang; status eksplisit `menunggu-rizal`, `menunggu`, `waiting`, `needs-action`, `butuh-rizal` → `MenungguRizal` bila `now - at < 2 jam`; `< 15 menit` → `Bekerja`; `< 2 jam` → `Berpikir`; peer online tanpa laporan → `Santai`; else `Offline` |
| Cadence | heartbeat event-driven; Tailscale 60000 ms; sweep TTL 30000 ms; seen 300000 ms; disappeared 7200000 ms (2 jam, sama dengan batas status) |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `log.recorded` |
| Dedupe | `heartbeat:muse:<at_ms>:<nonce>`; `seen:muse:<bucket>`; `disappeared:muse:<last_seen_ms>` |
| Level | L2 |
| Kontrol | `CommandQueue`; perintah diambil agent lewat `/api/v1/agents/agent:muse/commands` |
| Degradasi | Tidak ada heartbeat + peer offline → `Offline`; peer online tanpa heartbeat → `Santai`; Tailscale tidak ada → status dari heartbeat saja, catatan `tailscale_absent` |

### 3.9 Devin

| Item | Nilai |
|---|---|
| Sumber | API cloud `https://api.devin.ai/v1` (dapat di-override `HQ_DEVIN_BASE_URL`); `GET /sessions`; kredensial `DEVIN_API_KEY` atau `agent.devin.api_key` |
| Status mesin ini | Belum terverifikasi; tanpa kunci → `unconfigured`. |
| Parse | JSON sesi: `session_id`, `status`, `created_at`, `updated_at`, `title`, `url`, optional `usage`; status dipetakan: `running|working` → `Bekerja`, `blocked|waiting` → `MenungguRizal`, `finished|completed` → `Santai`, `failed|error` → `Error`, lainnya → `Berpikir`; field tak dikenal diabaikan |
| Cadence | poll 120000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `session.started`, `session.updated`, `session.ended`, `usage.recorded` bila `usage` ada, `log.recorded` untuk error |
| Dedupe | `session.started:devin/session/<id>:<created_at>`; `session.updated:...:<updated_at>`; `usage:devin:<session>:<provider>:<model>:<cursor_hash>` |
| Level | L2 |
| Kontrol | `HttpApi`; `POST /sessions/{id}/messages` lewat `hq-control`, capability `agent.prompt` |
| Degradasi | Tanpa kunci → `unconfigured`, tidak ada poll; 401/403 → `connector.auth_failed` + health `down`; respons rusak → `parse_error`, pertahankan cache terakhir; timeout → backoff, tidak menggantung |

### 3.10 MCP server

| Item | Nilai |
|---|---|
| Sumber config terverifikasi | `~/.omp/agent/mcp.json` (`mcpServers`); `~/.cursor/mcp.json` (`mcpServers`); `~/.codex/config.toml` (`[mcp_servers]`); `~/.gemini/settings.json` (`mcpServers`); `~/.config/opencode/opencode.jsonc` (`mcp`); `~/.copilot/mcp-config.json` (`mcpServers`) |
| Parse config | JSON/JSONC/TOML; nama server, command/args/env atau URL/headers; `disabledServers`/`enabled:false` dihormati; placeholder `${workspaceFolder}` atau `${env:PWD}` yang tidak bisa diselesaikan → server `unconfigured`, tidak di-spawn |
| Parse presence | Cocokkan basename command + argumen dengan proses; satu server bisa muncul di beberapa parent runtime; identitas tetap `mcp:<slug>` |
| Health handshake | Hanya untuk server stdio lokal dan HTTP yang punya URL; `tools/list` lewat `hq-mcp` dengan timeout 3000 ms; jumlah tool + hash nama tool disimpan; tidak memanggil tool |
| Cadence | config 60000 ms; presence 6000 ms; handshake 300000 ms; seen 300000 ms; disappeared 120000 ms |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `connector.event` dengan `event_type: "mcp.tools_observed"`, `connector.health_changed` |
| Dedupe | `observed:mcp:server:<slug>:<fingerprint>:<bucket>`; `mcp.tools_observed:<slug>:<tool_hash>`; `connector.health_changed:mcp:<slug>:<old>:<new>` |
| Level | L3 bila handshake berhasil; L1 bila hanya proses; `Offline` bila config ada tetapi tidak berjalan |
| Kontrol | `McpTool` lewat `hq-mcp`; default read-only; tool kontrol hanya dengan capability `McpControl` + approval (00 §12.0) |
| Degradasi | Config ada, proses tidak berjalan → `Offline`, catatan `configured_not_running`; handshake timeout → `Offline`; placeholder tak terselesaikan → `unconfigured`; config rusak → `Error` dengan `config_parse_error`, server lain tetap jalan |

Server yang sama (mis. `omni`) dikonfigurasi di banyak runtime tetap satu
subjek `agent:mcp/omni`; payload `data.parents` berisi daftar runtime yang
mengkonfigurasinya. Ini mencegah duplikasi agent MCP.

### 3.11 Heartbeat generik

| Item | Nilai |
|---|---|
| Sumber | `POST /api/v1/heartbeat` dengan `HeartbeatIn` (03 §1.2); file lama `HQ_DATA/heartbeats/<slug>.json` dimigrasikan sebagai input yang sama |
| Auth | Agent token per slug (`agent.<slug>.token` atau `HQ_AGENT_TOKEN_<SLUG>`), bukan token owner |
| Parse | `agent` slug, `at_ms`, `status`, `task`, `ttl_ms` (default 900000), `nonce`, optional `signature`; `task` di-redact lalu menjadi `Sentence`; `status` dipetakan ke enam status |
| Cadence | event-driven saat POST; sweep TTL 30000 ms; seen 300000 ms; disappeared = `ttl_ms` (default 900000 ms) |
| Event | `agent.observed`, `agent.status_changed`, `agent.seen`, `agent.disappeared`, `log.recorded` bila `task` berubah |
| Dedupe | `heartbeat:<slug>:<at_ms>:<nonce>`; `seen:<slug>:<bucket>`; `disappeared:<slug>:<last_seen_ms>` |
| Level | L1 tanpa task; L2 bila heartbeat membawa task/status eksplisit |
| Kontrol | `CommandQueue`; hasil lewat `POST /api/v1/agents/:id/commands/:cid/result` |
| Degradasi | Token salah → 401; tidak ada heartbeat > TTL → `agent.disappeared` sekali, status `Offline`, `last_seen_ms` dipertahankan; heartbeat tanpa signature hanya diterima bila token agent valid |

`subject` heartbeat adalah `agent:<slug>`, bukan `agent:heartbeat/<slug>`.
`runtime_key` tetap `heartbeat:<slug>` agar tidak bertabrakan dengan runtime
utama yang kebetulan punya slug sama.

## 4. Aturan volume event

Target 00 §9.4: **< 20.000 event/hari**. Angka ini adalah batas keras, bukan
saran. `LocalConnector` menghitung event yang akan dipancarkan dan menahan
kelas volume rendah bila anggaran harian hampir habis.

### 4.1 Kapan `agent.observed`, `agent.status_changed`, `agent.seen`

`AgentFingerprint` yang dibandingkan:

```text
status
activity.key
hash(activity.params)
process_bucket: 0 | 1 | 2 | 3-5 | 6+
evidence_level
provider
model
active_session_count
```

Tidak termasuk `cpu_percent`, `last_seen_ms`, atau nama tampilan.

| Kondisi | Event | Batas |
|---|---|---|
| Fingerprint berubah dan status berubah | `agent.status_changed` | maksimal 1 per `runtime_key` per 60 detik; perubahan ke/dari `Error` atau `MenungguRizal` diutamakan |
| Fingerprint berubah selain status | `agent.observed` | maksimal 1 per `runtime_key` per 60 detik; perubahan dalam 60 detik digabung, hanya nilai akhir |
| Tidak ada perubahan, ada bukti presence | `agent.seen` | maksimal 1 per `runtime_key` per 300 detik; **tidak** dipancarkan bila `agent.observed`/`status_changed` sudah keluar di bucket 300 detik yang sama |
| Presence hilang melewati `disappeared_after_ms` | `agent.disappeared` | sekali per episode; `last_seen_ms` di payload |
| Presence kembali | `agent.observed` | sekali, status baru |

`agent.seen` dan `agent.observed` berbagi satu bucket 300 detik per agent.
Dengan 20 subjek agent aktif, plafonnya 20 × 288 = 5.760 event/hari.

### 4.2 Sesi

| Event | Kondisi | Batas / dedupe |
|---|---|---|
| `session.started` | session id pertama kali terlihat | sekali per `(session_subject, started_ms)` |
| `session.updated` | fingerprint sesi berubah: title, model, last tool, ended, usage bucket | maksimal 1 per sesi per 300 detik; key `session.updated:<subject>:<source_cursor_hash>:<bucket>` |
| `session.ended` | marker ended/archived/exit atau sumber hilang > TTL | sekali per `(session_subject, ended_ms)` |

Fingerprint sesi: `ended`, `title.key + hash(params)`, `last_tool`, `model`,
`usage_bucket` (token dibulatkan 1.000, biaya 3 significant figures).

### 4.3 Usage dan kuota

- `usage.recorded` adalah **delta**, bukan kumulatif. `LocalConnector`
  menyimpan baseline kumulatif per `(runtime_key, session, provider, model,
  source_cursor)` di `AdapterCache`.
- Delta positif > 0 → satu event per 300 detik per `(agent, provider, model)`.
  Delta dalam 300 detik digabung.
- Delta negatif (file/DB reset) → **tidak** memancarkan delta negatif. Baseline
  di-reset ke nilai baru, `reset: true` dicatat di cache dan `EvidenceNote`.
  Ini mencegah double count setelah truncation; kehilangan usage selama
  truncation dinyatakan, bukan dikarang.
- `quota.observed` hanya saat `percent` berubah ≥ 0,1 poin atau `resets_ms`
  berubah; maksimal 1 per 300 detik per provider.
- `cost.estimated` bukan event adapter; projector yang menghitung dari
  `usage.recorded` dan tabel harga.

Key:

```text
usage.recorded : usage:<runtime_key>:<session|agent>:<provider>:<model>:<cursor_hash>
quota.observed : quota:<runtime_key>:<provider>:<label_slug>:<percent_x10>:<reset_bucket>
```

### 4.4 Log

`log.recorded` hanya untuk:

- error/warning dari transcript, stderr tool, exit non-zero, atau health error;
- transisi task lifecycle yang sudah ditentukan humanizer (mulai, menunggu,
  selesai, gagal);
- bukan setiap baris output, bukan setiap tool call sukses.

Batas: maksimal 20 event per sesi per 300 detik, dan maksimal 1 per sesi per
30 detik kecuali error. `raw` hanya diisi bila `HQ_LOG_RAW=1`; nilainya sudah
di-redact dan dipotong 4 KiB.

Key: `log:<runtime_key>:<session|agent>:<cursor_hash>`.

### 4.5 Anggaran harian

| Kelas | Plafon | Dasar |
|---|---|---|
| `agent.seen` + `agent.observed` + `agent.status_changed` | 5.760 | 20 agent × 288 bucket 5 menit |
| `session.started` + `session.updated` + `session.ended` | 3.000 | 20 sesi aktif, update 5 menit |
| `usage.recorded` + `quota.observed` | 2.500 | 20 agent × model, delta 5 menit |
| `log.recorded` | 1.000 | hanya error/wait + batas burst |
| `connector.health_changed` + `connector.poll_completed` | 1.000 | 12 connector, poll_completed di-cap 1/menit |
| `system.health_observed`, `config.changed`, lain-lain | 500 | operasional |
| Cadangan alert/action/notification | 1.000 | tidak boleh dipakai kelas volume |
| **Total** | **14.760 tipikal; 20.000 keras** | token bucket per hari UTC |

Bila token bucket < 20% tersisa, urutan penurunan: `agent.seen` →
`session.updated` non-penting → `log.recorded` non-error → `agent.observed`
non-prioritas. `alert.*`, `action.*`, `usage.recorded`, `connector.auth_failed`,
dan `agent.disappeared` tidak pernah diturunkan. Setiap penurunan dicatat di
metrik `hq_events_dropped_total{reason}` dan `/health`, bukan event.

## 5. Parity harness

Harness mengikuti [`08-testing-gates.md`](08-testing-gates.md) §3. Dokumen ini
menetapkan bentuk artefak dan normalizer yang dipakai S09, S10, dan S25.

### 5.1 Artefak beku S00

`tools/parity/freeze.sh` menghasilkan:

| Path | Isi |
|---|---|
| `tools/parity/oracle/rizal-agent-hq-v2` | binary v2 + `VERSION` |
| `tools/parity/oracle/rizal-agent-hq.service` | unit systemd asli |
| `tools/parity/fixtures/state-<ts>.json` | 200 sampel `GET /api/state`, interval 6 detik |
| `tools/parity/fixtures/raw-<ts>/` | 20 sampel pertama: `ps.txt`, `omp-stats.json`, `sessions.txt`, `tmux.txt`, `docker.txt`, `git.txt` |
| `tools/parity/fixtures/aliases.json` | alias v2 → v3 |
| `tools/parity/MANIFEST.sha256` | checksum binary + semua fixture |

`tools/parity/verify.sh` mengembalikan 0 bila checksum cocok. Fixture beku
menguji normalizer dan humanizer; uji yang mengikat tetap live 8900 vs 8901.

### 5.2 Bentuk normalizer

```json
{
  "schema_version": 1,
  "generated_at_ms": 1794844000000,
  "window": {
    "start_ms": 1794843400000,
    "end_ms": 1794844000000,
    "duration_s": 600,
    "interval_s": 5
  },
  "agents": [
    {
      "id": "agent:omp",
      "v2_id": "omp",
      "status": "bekerja",
      "evidence": "L3",
      "processes": 2,
      "usage": {
        "requests": 325,
        "input_tokens": 108000000,
        "output_tokens": 584000,
        "cache_tokens": 0,
        "cash_usd": 1.481,
        "shadow_usd": 0.0
      },
      "aliased": true,
      "extra": false,
      "samples": 120,
      "status_match": 0.94
    }
  ],
  "alerts": [
    {
      "rule_id": "agent.waiting",
      "subject": "agent:muse",
      "level": "warning",
      "fingerprint": "agent.waiting|agent:muse|warning",
      "state": "open"
    }
  ],
  "usage": {
    "global": {
      "requests": 1000,
      "input_tokens": 200000000,
      "output_tokens": 1000000,
      "cache_tokens": 0,
      "cash_usd": 4.2,
      "shadow_usd": 1.1
    },
    "by_agent": []
  },
  "unmatched_v2": [],
  "extra_v3": [],
  "counts": {
    "samples": 120,
    "agents_v2": 5,
    "agents_v3": 5
  }
}
```

Aturan normalisasi:

- Agent dipetakan lewat `aliases.json`; `v2_id` dipertahankan untuk laporan.
- Urutkan agent by `id`; buang field waktu tampilan.
- Usage dibulatkan 3 significant figures.
- Status dinormalisasi ke enam nilai 01 §2.
- Alert dibandingkan `(rule_id, subject, level)`, bukan id acak.
- `extra: true` hanya untuk agent v3 tanpa padanan v2 yang punya bukti
  proses/file; `extra_v3` mencatat id-nya.
- Sampel status dihitung per agent: `status_match = cocok / samples`.

### 5.3 Kriteria lulus

| Kriteria | Ambang | Aksi bila gagal |
|---|---|---|
| Cakupan | setiap agent v2 punya padanan v3 (`v2 ⊆ v3`) | `FAIL`; tambah alias/adapter |
| Status | cocok ≥ 90% sampel untuk agent berpasangan | `FAIL`; perbaiki precedence status |
| Usage global | selisih ≤ 15% pada jendela waktu sama | `FAIL`; periksa delta usage |
| Alert | fingerprint set sama untuk alert yang masih open | `FAIL`; periksa rule/projection |
| Tambahan | agent baru hanya bila ada bukti runtime | `FAIL` secara default; warning bila `--allow-extra` |

### 5.4 Perintah dan flag

```bash
hq parity \
  --oracle http://127.0.0.1:8900 \
  --candidate http://127.0.0.1:8901 \
  --duration 600 \
  --interval 5 \
  --aliases tools/parity/fixtures/aliases.json \
  --out tools/parity/report-$(date +%s).md
```

| Flag | Default | Arti |
|---|---|---|
| `--oracle <url>` | wajib | v2 live |
| `--candidate <url>` | wajib | v3 live |
| `--duration <s>` | 600 | durasi sampling simultan |
| `--interval <s>` | 5 | jarak sampel; minimum 1 |
| `--aliases <path>` | `tools/parity/fixtures/aliases.json` | alias file |
| `--out <path>` | wajib | laporan Markdown |
| `--allow-extra` | nonaktif | tanpa flag: agent v3 tambahan tanpa bukti = `FAIL`; dengan flag: warning, dipakai subset S09/S10 |

### 5.5 Format laporan

```markdown
# Parity report

- Verdict: PASS
- Oracle: http://127.0.0.1:8900
- Candidate: http://127.0.0.1:8901
- Window: 2026-11-16T22:00:00Z .. 2026-11-16T22:10:00Z
- Samples: 120

## Coverage

| v2 | v3 | Status |
|---|---|---|
| omp | agent:omp | matched |

## Status

| Agent | Match | Threshold |
|---|---|---|
| agent:omp | 94% | 90% |

## Usage

| Scope | v2 | v3 | Diff |
|---|---|---|---|
| global | 4.20 USD | 4.31 USD | +2.6% |

## Alerts

| Rule | Subject | Level | Result |
|---|---|---|---|

## Extra agents

| Agent | Evidence | Result |
|---|---|---|

## Failures

- Tidak ada.
```

### 5.6 Alias file

```json
{
  "schema_version": 1,
  "aliases": {
    "omp": "agent:omp",
    "cursor": "agent:cursor",
    "Muse": "agent:muse",
    "mcp": "agent:mcp",
    "omp-workers": "agent:omp/worker/agg",
    "omp-workers-agg": "agent:omp/worker/agg",
    "claude-code": "agent:claude-code",
    "codex": "agent:codex",
    "devin": "agent:devin"
  },
  "dynamic": [
    {
      "v2_prefix": "omp-s-",
      "v3_template": "agent:omp/session/{full_id}"
    },
    {
      "v2_prefix": "omp-w-",
      "v3_template": "agent:omp/worker/{full_id}"
    }
  ]
}
```

Nama tes yang wajib ada (08 §2, Review Focus #1):
`collector_survives_disappearing_source` di S04, S09, dan S10. Tes normalizer:
`parity_normalizer_maps_v2_aliases`, `parity_normalizer_keeps_extra_agent`,
`parity_report_fails_below_status_threshold`,
`parity_report_fails_on_usage_drift`.

## 6. Timeout, isolasi kegagalan, cache, dan sumber yang menghilang

### 6.1 Timeout

| Operasi | Timeout | Cadence | Perilaku gagal |
|---|---|---|---|
| `ps` | 1.500 ms | 6.000 ms | presence tidak berubah; 2 miss → `Offline` |
| `tmux` | 1.000 ms | 6.000 ms | sesi tmux dipertahankan dari cache |
| `git` | 2.000 ms | 60.000 ms | commit terakhir dipertahankan |
| `docker` | 2.000 ms | 15.000 ms | container list dipertahankan |
| `tailscale` | 2.000 ms | 60.000 ms | peer `unknown`, bukan offline paksa |
| JSONL tail | 500 ms | 30.000 ms | offset tidak maju; retry tick berikut |
| SQLite query | 1.000 ms (`busy_timeout` 250 ms) | 30.000 ms | cache dipakai; health `degraded` |
| `omp stats --json` | 45.000 ms | 300.000 ms | cache dipakai; catat `timeout` |
| `omp usage` | 45.000 ms | 300.000 ms | cache dipakai; catat `timeout` |
| MCP `tools/list` | 3.000 ms | 300.000 ms | server `Offline`; child dibunuh |
| Devin HTTP | 20.000 ms total | 120.000 ms | backoff; cache terakhir |
| Heartbeat POST | 2.000 ms | event | 401/429; tidak menyimpan task |
| `LocalConnector::poll` | `min(HQ_INTERVAL*1000 - 500, 5.000)` ms | setiap tick | partial result; tidak menggantung |

`observe` tidak boleh memanggil perintah 45 detik. Semua perintah mahal hanya
di `refresh`; `observe` membaca cache. Ini yang menjaga snapshot tetap selesai
meskipun `omp stats` lambat.

### 6.2 Isolasi kegagalan

- Setiap adapter berjalan di task `JoinSet` sendiri; timeout per adapter.
- Satu adapter error/panic → `Observation` kosong + `EvidenceNote`; adapter lain
  tetap menghasilkan event.
- Command child selalu `kill()` pada timeout; tidak ada proses zombie.
- Operasi blocking (SQLite, baca file besar) lewat `tokio::task::spawn_blocking`
  atau berjalan di dalam `observe` dengan batas baca 1 MiB per file per tick.
- Tidak ada `unwrap` pada input file, DB, proses, atau HTTP (01 §8).
- Error adapter tidak pernah memuat isi prompt, header, token, atau argv penuh.

### 6.3 Cache

| Path | Isi | Batas |
|---|---|---|
| `HQ_DATA/adapters/<runtime>/state.json` | cursor file/DB, fingerprint, baseline usage, `last_refresh_ms` | 1 MiB |
| `HQ_DATA/adapters/<runtime>/stats.json` | hasil `omp stats`/`omp usage` terakhir | 256 KiB |
| `HQ_DATA/adapters/<runtime>/quota.json` | kuota terakhir | 64 KiB |

- Tulis atomik; mode 0600; fsync file + parent dir.
- File rusak → rename `*.corrupt-<ts>`, mulai kosong, catat `cache_recovered`.
- Cache tidak pernah berisi secret. Cursor boleh berisi path; path di-redact
  bila mengandung pola token.
- Cache dibersihkan untuk sesi > 30 hari pada startup.

### 6.4 Sumber yang menghilang

| Kejadian | Perilaku |
|---|---|
| Proses mati | presence false; setelah `disappeared_after_ms` (default 120.000 ms) `agent.disappeared` sekali; status `Offline`; `last_seen_ms` dan usage terakhir dipertahankan |
| File transcript hilang | offset/cache dipertahankan; status dari proses; setelah TTL `Offline`; jika file kembali, parse dari offset terakhir atau awal bila inode berubah |
| File terpotong | deteksi `size < offset` atau inode berubah; reset offset; baseline usage di-reset; **tidak** ada delta negatif; catat `source_reset` |
| Baris JSONL terpotong | simpan offset di awal baris; jangan parse baris tanpa newline; retry setelah file tumbuh |
| SQLite hilang/terkunci | cache dipakai; health `degraded`; tidak ada event palsu; proses tetap memberi presence |
| Binary hilang | perintah tidak dijalankan; `AdapterError::Unavailable { code: "binary_absent" }`; data file tetap dipakai dengan catatan |
| Hanya config | `Offline` dengan catatan `config_only`; tidak ada klaim task; tidak ada agent baru bila runtime opsional |
| HTTP 401 | `connector.auth_failed`; health `down`; tidak retry dengan kredensial sama sampai `next_run_ms` |
| HTTP 5xx/timeout | backoff + jitter; cache terakhir; tidak menggantung |

Snapshot tidak pernah menunggu sumber yang hilang. Semua jalur di atas
menghasilkan hasil dalam deadline `poll`.

## 7. File dan tes per sesi

### 7.1 S04 — kontrak, proses, heartbeat

**Create**

- `crates/hq-connectors/src/runtime/mod.rs`
- `crates/hq-connectors/src/runtime/cache.rs`
- `crates/hq-connectors/src/runtime/proc.rs`
- `crates/hq-connectors/src/runtime/heartbeat.rs`
- `crates/hq-connectors/src/identity.rs`
- `crates/hq-connectors/src/native/local.rs`
- `crates/hq-connectors/tests/runtime_contract.rs`
- `crates/hq-connectors/tests/runtime_heartbeat.rs`
- `crates/hq-connectors/tests/fixtures/processes/*.txt`
- `crates/hq-connectors/tests/fixtures/heartbeat/*.json`

**Tes wajib**

| Tes | Acceptance |
|---|---|
| `collector_survives_disappearing_source` | proses hilang di tengah poll; snapshot selesai < deadline; `agent.disappeared` sekali; status `Offline`; `last_seen_ms` ada |
| `local_connector_never_exceeds_deadline` | satu adapter `sleep` melebihi timeout; `poll` tetap selesai; adapter lain tetap menghasilkan event |
| `heartbeat_ttl_marks_disappeared` | heartbeat lewat TTL → `agent.disappeared` sekali, bukan setiap sweep |
| `heartbeat_duplicate_nonce_is_noop` | POST dengan nonce sama → satu event, `connector_runs.duplicates = 1` |
| `identity_alias_v2_maps_to_v3` | lima alias v2 → subject v3 yang benar |
| `process_probe_redacts_args` | argv berisi token/JWT tidak muncul di event/log |
| `adapter_cache_atomic_and_permissions` | file cache 0600, tulis ulang tidak korup |

### 7.2 S09 — OMP, Cursor, Muse

**Create**

- `crates/hq-connectors/src/runtime/omp.rs`
- `crates/hq-connectors/src/runtime/cursor.rs`
- `crates/hq-connectors/src/runtime/muse.rs`
- `crates/hq-connectors/tests/runtime_omp.rs`
- `crates/hq-connectors/tests/runtime_cursor.rs`
- `crates/hq-connectors/tests/runtime_muse.rs`
- `crates/hq-connectors/tests/fixtures/omp/session-*.jsonl`
- `crates/hq-connectors/tests/fixtures/omp/stats.db`
- `crates/hq-connectors/tests/fixtures/cursor/projects/**/*.jsonl`
- `crates/hq-connectors/tests/fixtures/muse/heartbeat.json`
- `crates/hq-connectors/tests/fixtures/muse/tailscale.json`

**Tes wajib**

| Tes | Acceptance |
|---|---|
| `omp_stats_and_sessions_merge_one_agent` | proses + session + usage menghasilkan satu `agent:omp`, bukan tiga |
| `omp_session_jsonl_incremental_tail` | baris baru diproses sekali; offset tidak mundur |
| `omp_usage_delta_emitted_once` | `model_usage` sama dibaca dua kali → satu `usage.recorded` |
| `omp_subagent_uses_parent` | subagent jadi `agent:omp/worker/<id>` dengan parent `agent:omp` |
| `cursor_workspace_identity_stable` | path sama → hash sama; path berubah → hash berubah |
| `cursor_transcript_absent_is_l1` | hanya state file/proses → L1, bukan L2 |
| `muse_heartbeat_waiting_overrides_age` | status `menunggu` < 2 jam → `MenungguRizal` |
| `muse_tailscale_offline_is_offline` | peer offline + heartbeat tua → `Offline` |
| `collector_survives_disappearing_source` | OMP/Cursor/Muse hilang saat poll; tidak menggantung |
| `parity_subset_maps_v2_agents` | subset omp/cursor/Muse lulus alias + status ≥ 90% |

### 7.3 S10 — Claude, Codex, Gemini, OpenCode, Copilot, Devin, MCP

**Create**

- `crates/hq-connectors/src/runtime/claude_code.rs`
- `crates/hq-connectors/src/runtime/codex.rs`
- `crates/hq-connectors/src/runtime/gemini_cli.rs`
- `crates/hq-connectors/src/runtime/opencode.rs`
- `crates/hq-connectors/src/runtime/copilot.rs`
- `crates/hq-connectors/src/runtime/devin.rs`
- `crates/hq-connectors/src/runtime/mcp.rs`
- `crates/hq-connectors/tests/runtime_claude.rs`
- `crates/hq-connectors/tests/runtime_codex.rs`
- `crates/hq-connectors/tests/runtime_gemini.rs`
- `crates/hq-connectors/tests/runtime_opencode.rs`
- `crates/hq-connectors/tests/runtime_copilot.rs`
- `crates/hq-connectors/tests/runtime_devin.rs`
- `crates/hq-connectors/tests/runtime_mcp.rs`
- `crates/hq-connectors/tests/fixtures/{claude,codex,gemini,opencode,copilot,devin,mcp}/`
- `crates/hq-connectors/tests/fixtures/opencode/opencode.db` (skema + 3 sesi)
- `crates/hq-connectors/tests/fixtures/mcp/*.json` (config per runtime)

**Tes wajib**

| Tes | Acceptance |
|---|---|
| `claude_config_only_is_not_activity` | hanya `~/.claude/settings.json` → `Offline`, catatan `config_only`, tidak ada task |
| `claude_transcript_usage_parses` | fixture JSONL → session + token usage; level L3 |
| `codex_rollout_incremental` | rollout baru diproses sekali; token `total_token_usage` benar |
| `codex_absent_is_offline` | tanpa rollout/proses → `Offline`, bukan agen aktif |
| `gemini_config_only_is_offline` | `~/.gemini` tanpa sesi Gemini CLI → `Offline`; `antigravity-*` diabaikan |
| `opencode_usage_delta_once` | query berulang → satu delta; biaya/token cocok |
| `opencode_schema_mismatch_is_error` | DB tanpa kolom usage → `Error` `schema_mismatch`, bukan data palsu |
| `copilot_shim_is_l1` | shim Windows + log kosong → L1 |
| `devin_without_key_is_unconfigured` | tanpa `DEVIN_API_KEY` → `unconfigured`, tidak ada request |
| `devin_401_marks_auth_failed` | 401 → `connector.auth_failed`, health `down` |
| `mcp_server_merges_across_runtimes` | `omni` di 3 config → satu `agent:mcp/omni` |
| `mcp_placeholder_is_unconfigured` | `${workspaceFolder}` tak terselesaikan → `unconfigured`, tidak spawn |
| `mcp_handshake_timeout_is_offline` | `tools/list` timeout → `Offline`, child mati |
| `collector_survives_disappearing_source` | DB/file/proses hilang saat poll; snapshot selesai |
| `parity_runtime_b_maps_v2_agents` | alias + status ≥ 90%; agent baru hanya dengan bukti |

## Diserahkan ke builder

- Hash Cursor untuk direktori project berbentuk hash tidak bisa dibalik menjadi
  path absolut; dokumen ini memilih fallback `sha1("cursor-project:"+slug)`.
  Jika builder menemukan pemetaan resmi Cursor, ganti fallback itu.
- Subjek sintetis `agent:omp/worker/agg` hanya untuk paritas. Builder memilih
  apakah ia muncul di UI sebagai baris agregat atau disembunyikan dari Today.
- Format transcript Claude Code, Codex, dan Gemini CLI di mesin ini tidak punya
  sampel (binary tidak ada). Parser wajib toleran; builder mengunci versi
  skema saat sampel nyata pertama tersedia, tanpa menambah klaim level.
- Apakah `agent.seen` dipancarkan untuk agent yang sudah `Offline`; dokumen ini
  memilih tidak, karena `Offline` hanya berubah lewat `agent.disappeared` dan
  `agent.observed`.
