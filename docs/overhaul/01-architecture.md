# 01 — Architecture Contracts

Kontrak yang tidak boleh diubah tanpa memperbarui dokumen ini dan semua sesi
yang memakainya. Design lengkap: [`00-design.md`](00-design.md).

## 1. Workspace

```
Cargo.toml                      # workspace, resolver 3, edition 2024
rust-toolchain.toml             # channel = "1.96", components fmt/clippy
deny.toml
crates/
  hq-core/
  hq-store/
  hq-connectors/
  hq-control/
  hq-alerts/
  hq-humanize/
  hq-knowledge/
  hq-observe/
  hq-mcp/
  hq-api/
  hq-server/
web/                            # SvelteKit
tools/parity/                   # oracle + fixture + normalizer
docs/overhaul/
```

Aturan dependensi (ditegakkan review, bukan hanya konvensi):

| Crate | Boleh bergantung pada | Tidak boleh |
|---|---|---|
| `hq-core` | serde, ulid, thiserror, schemars | tokio, rusqlite, axum, reqwest |
| `hq-store` | core, rusqlite, tokio (channel), tracing | axum, reqwest |
| `hq-connectors` | core, reqwest, tokio, jsonschema | store, api, rusqlite |
| `hq-control` | core, tokio, tracing | store langsung (audit lewat trait), api |
| `hq-alerts` | core, tokio | store langsung, reqwest |
| `hq-humanize` | core | I/O apa pun |
| `hq-knowledge` | core, store, fastembed | axum, reqwest, menulis event |
| `hq-observe` | core, opentelemetry, prometheus-client | store, api |
| `hq-mcp` | core, rmcp, tokio | store langsung |
| `hq-api` | core, store, connectors, control, alerts, humanize, knowledge, observe | rusqlite langsung |
| `hq-server` | semua | logika domain baru |

`hq-server` hanya wiring: baca config, buka store, daftarkan connector, jalankan
supervisor, pasang router. Tidak ada aturan bisnis di sini.

## 2. Tipe inti (`hq-core`)

```rust
// ids.rs
pub type Ms = i64;
pub struct Seq(pub i64);
pub struct AgentId(pub String);      // "agent:omp"
pub struct SessionId(pub String);
pub struct TaskId(pub String);
pub struct AlertId(pub String);
pub struct ConnectorId(pub String);
pub struct DeviceId(pub String);
pub struct ActionId(pub String);
pub struct EventId(pub Ulid);

// subject.rs
pub enum Subject {
    Agent(AgentId),
    Session(SessionId),
    Connector(ConnectorId),
    Alert(AlertId),
    Device(DeviceId),
    Action(ActionId),
    Knowledge(String),
    Usage(UsageScope),
    System,
}
pub enum UsageScope { Global, Agent(AgentId), Session(SessionId) }

impl Subject {
    /// Bentuk kanonik yang disimpan di kolom `subject`.
    pub fn key(&self) -> String;
}
```

Bentuk kanonik: `agent:omp`, `agent:omp/session/<id>`,
`agent:omp/worker/<id>`, `agent:cursor/workspace/<hash>`, `connector:github`,
`alert:<id>`, `device:<id>`, `action:<id>`, `knowledge:<source>:<page>`,
`usage:global`, `usage:agent:<id>`, `system`.

```rust
// event.rs
pub struct EventDraft {
    pub event_ts_ms: Ms,
    pub kind: EventKind,
    pub subject: Subject,
    pub source: SourceId,
    pub correlation_id: Option<EventId>,
    pub causation_id: Option<EventId>,
    pub idempotency_key: Option<String>,
    pub payload: serde_json::Value,
}

pub struct Event {
    pub seq: Seq,
    pub id: EventId,
    pub ingest_ts_ms: Ms,
    pub draft: EventDraft,
}

pub struct SourceId(pub String); // "local", "connector:github", "legacy", "user"
```

`EventKind` adalah enum tertutup untuk domain inti:

```rust
pub enum EventKind {
    AgentObserved, AgentSeen, AgentStatusChanged, AgentDisappeared,
    SessionStarted, SessionUpdated, SessionEnded,
    LogRecorded,
    UsageRecorded, QuotaObserved, CostEstimated,
    AlertRaised, AlertUpdated, AlertAcknowledged, AlertSnoozed, AlertResolved,
    ActionRequested, ActionApproved, ActionDenied, ActionExecuted, ActionFailed,
    ConnectorRegistered, ConnectorHealthChanged, ConnectorPollCompleted,
    ConnectorAuthFailed, ConnectorEvent,
    KnowledgePageSynced, KnowledgePageQuarantined, KnowledgePromoted,
    SystemHealthObserved, ConfigChanged, SecretRotated,
    DeviceAdded, DeviceRevoked, NotificationSent,
    TelemetryMetric, TelemetryTrace, TelemetryLog,
}
```

Payload per kind adalah struct di `hq-core::payload`, derive `Serialize`,
`Deserialize`, `JsonSchema`. Validasi memakai `jsonschema` terhadap skema yang
dihasilkan `schemars`, disimpan di `hq-core::schema::schema_for(kind, version)`.

```rust
// sentence.rs
pub struct Sentence {
    pub key: String,                 // "task.running_tests"
    pub params: BTreeMap<String, String>,
}

// agent.rs
pub enum Status { Bekerja, Berpikir, MenungguRizal, Santai, Offline, Error }
pub enum AgentKind { Utama, Worker, Mcp }
pub enum EvidenceLevel { L1, L2, L3, L4 }

// task.rs
pub enum TaskPhase { Antre, Berjalan, MenungguRizal, Selesai, Gagal, Batal }

// alert.rs
pub enum AlertLevel { Info, Warning, Critical }
pub enum AlertState { Open, Acknowledged, Snoozed, Resolved }

// control.rs
pub enum Actor { Owner, Automation(String), Agent(AgentId), System }
pub enum Capability {
    AgentPrompt, AgentPause, AgentResume, AgentStop,
    ToolApprove, ToolDeny, ConnectorAction(String),
    ShellRun(String), ConfigWrite, DeviceManage, McpControl,
}
pub enum PolicyDecision { Allow, RequireApproval, Deny }

// usage.rs
pub enum Attribution { Exact, Global, Estimated }
```

Status diserialisasi kebab-case (`menunggu-rizal`) dan dirender lewat katalog
i18n; tidak ada teks status yang disimpan.

## 3. Store (`hq-store`)

```rust
pub struct StoreOptions { pub path: PathBuf, pub read_pool: usize, pub busy_timeout_ms: u64 }
pub struct Store(/* clone-able handle */);

impl Store {
    pub async fn open(opts: StoreOptions) -> Result<Store, StoreError>;
    pub async fn append(&self, drafts: Vec<EventDraft>) -> Result<Vec<Event>, StoreError>;
    pub async fn read_since(&self, since: Seq, limit: usize) -> Result<Vec<Event>, StoreError>;
    pub async fn query_events(&self, q: EventQuery) -> Result<EventPage, StoreError>;
    pub async fn overview(&self) -> Result<Overview, StoreError>;
    pub async fn agent(&self, id: &AgentId, as_of: Option<Ms>) -> Result<Option<AgentDetail>, StoreError>;
    pub async fn alerts(&self, filter: AlertFilter) -> Result<Vec<AlertView>, StoreError>;
    pub async fn usage(&self, q: UsageQuery) -> Result<UsageReport, StoreError>;
    pub fn patches(&self) -> broadcast::Receiver<Patch>;
    pub async fn rebuild(&self, opts: RebuildOptions) -> Result<RebuildReport, StoreError>;
    pub async fn backup(&self, dest: &Path) -> Result<(), StoreError>;
}
```

Semantik:

- `append`: menulis event + projection inti dalam **satu transaksi**; setelah
  commit mengirim `Patch` ke broadcast. `seq` dialokasikan SQLite.
- `read_since`: membaca `events` + `events_archive` berurutan `seq`.
- `agent(as_of)`: replay event subject tersebut dari checkpoint terdekat ke
  tabel sementara; tidak mengubah read model.
- `rebuild`: menulis ke database bayangan, lalu swap file; store tetap melayani
  read dari DB lama sampai swap.
- `backup`: `VACUUM INTO` + salinan config; aman saat live.

Skema awal (`migrations/0001_init.sql`):

```sql
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA busy_timeout=5000;
PRAGMA foreign_keys=ON;

CREATE TABLE events (
  seq            INTEGER PRIMARY KEY AUTOINCREMENT,
  id             TEXT NOT NULL UNIQUE,
  event_ts_ms    INTEGER NOT NULL,
  ingest_ts_ms   INTEGER NOT NULL,
  kind           TEXT NOT NULL,
  subject        TEXT NOT NULL,
  source         TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  correlation_id TEXT,
  causation_id   TEXT,
  idempotency_key TEXT,
  payload        TEXT NOT NULL
);
CREATE INDEX events_ts      ON events(event_ts_ms);
CREATE INDEX events_subject ON events(subject, event_ts_ms);
CREATE INDEX events_kind    ON events(kind, event_ts_ms);
CREATE INDEX events_corr    ON events(correlation_id);
CREATE UNIQUE INDEX events_idem ON events(source, idempotency_key)
  WHERE idempotency_key IS NOT NULL;

CREATE TABLE agents (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  kind TEXT NOT NULL,
  parent_id TEXT,
  runtime_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL,
  activity_json TEXT NOT NULL,
  provider TEXT,
  model TEXT,
  requests INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_tokens INTEGER NOT NULL DEFAULT 0,
  cash_usd REAL NOT NULL DEFAULT 0,
  shadow_usd REAL NOT NULL DEFAULT 0,
  processes INTEGER NOT NULL DEFAULT 0,
  cpu_percent REAL NOT NULL DEFAULT 0,
  evidence TEXT NOT NULL,
  last_seen_ms INTEGER NOT NULL,
  updated_seq INTEGER NOT NULL
);

CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  runtime TEXT NOT NULL,
  cwd TEXT,
  started_ms INTEGER NOT NULL,
  ended_ms INTEGER,
  outcome TEXT,
  summary_json TEXT,
  requests INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_tokens INTEGER NOT NULL DEFAULT 0,
  cash_usd REAL NOT NULL DEFAULT 0,
  shadow_usd REAL NOT NULL DEFAULT 0,
  updated_seq INTEGER NOT NULL
);
CREATE INDEX sessions_agent ON sessions(agent_id, started_ms DESC);

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  session_id TEXT,
  title_json TEXT NOT NULL,
  phase TEXT NOT NULL,
  started_ms INTEGER NOT NULL,
  updated_ms INTEGER NOT NULL,
  ended_ms INTEGER,
  outcome_json TEXT,
  ordinal INTEGER NOT NULL DEFAULT 0,
  updated_seq INTEGER NOT NULL
);
CREATE INDEX tasks_agent ON tasks(agent_id, updated_ms DESC);
CREATE INDEX tasks_phase ON tasks(phase, updated_ms DESC);

CREATE TABLE task_steps (
  task_id TEXT NOT NULL,
  idx INTEGER NOT NULL,
  label_json TEXT NOT NULL,
  done INTEGER NOT NULL DEFAULT 0,
  at_ms INTEGER,
  PRIMARY KEY (task_id, idx)
);

CREATE TABLE logs (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  session_id TEXT,
  ts_ms INTEGER NOT NULL,
  level TEXT NOT NULL,
  sentence_json TEXT NOT NULL,
  raw TEXT,
  seq INTEGER NOT NULL
);
CREATE INDEX logs_agent ON logs(agent_id, ts_ms DESC);

CREATE VIRTUAL TABLE logs_fts USING fts5(
  log_id UNINDEXED, text, tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE alerts (
  id TEXT PRIMARY KEY,
  fingerprint TEXT NOT NULL UNIQUE,
  rule_id TEXT NOT NULL,
  level TEXT NOT NULL,
  state TEXT NOT NULL,
  subject TEXT NOT NULL,
  title_json TEXT NOT NULL,
  detail_json TEXT NOT NULL,
  opened_ms INTEGER NOT NULL,
  updated_ms INTEGER NOT NULL,
  ack_ms INTEGER,
  snooze_until_ms INTEGER,
  resolved_ms INTEGER,
  occurrences INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE alert_decisions (
  alert_id TEXT NOT NULL,
  actor TEXT NOT NULL,
  action TEXT NOT NULL,
  at_ms INTEGER NOT NULL,
  note TEXT
);

CREATE TABLE usage_minute (
  bucket_ms INTEGER NOT NULL,
  agent_id TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  attribution TEXT NOT NULL,
  requests INTEGER NOT NULL,
  input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL,
  cache_tokens INTEGER NOT NULL,
  cash_usd REAL NOT NULL,
  shadow_usd REAL NOT NULL,
  PRIMARY KEY (bucket_ms, agent_id, provider, model, attribution)
);
CREATE TABLE usage_daily (
  day TEXT NOT NULL, agent_id TEXT NOT NULL, provider TEXT NOT NULL,
  model TEXT NOT NULL, attribution TEXT NOT NULL,
  requests INTEGER NOT NULL, input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL, cache_tokens INTEGER NOT NULL,
  cash_usd REAL NOT NULL, shadow_usd REAL NOT NULL,
  PRIMARY KEY (day, agent_id, provider, model, attribution)
);

CREATE TABLE system_health (
  ts_ms INTEGER PRIMARY KEY,
  cpu_percent REAL NOT NULL,
  mem_percent REAL NOT NULL,
  disk_percent REAL NOT NULL,
  uptime_secs INTEGER NOT NULL,
  docker_json TEXT,
  git_json TEXT,
  updated_seq INTEGER NOT NULL
);

CREATE TABLE connectors (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, tier TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0, health TEXT NOT NULL,
  last_ok_ms INTEGER, last_error TEXT, next_run_ms INTEGER,
  config_path TEXT, updated_seq INTEGER NOT NULL
);
CREATE TABLE connector_runs (
  id TEXT PRIMARY KEY, connector_id TEXT NOT NULL,
  started_ms INTEGER NOT NULL, finished_ms INTEGER,
  ok INTEGER, events INTEGER NOT NULL DEFAULT 0,
  duplicates INTEGER NOT NULL DEFAULT 0, error TEXT
);

CREATE TABLE approvals (
  id TEXT PRIMARY KEY, action_id TEXT NOT NULL,
  capability TEXT NOT NULL, target TEXT NOT NULL,
  input_json TEXT NOT NULL, state TEXT NOT NULL,
  requested_by TEXT NOT NULL, requested_ms INTEGER NOT NULL,
  expires_ms INTEGER NOT NULL, decided_by TEXT, decided_ms INTEGER,
  note TEXT
);

CREATE TABLE audits (
  id TEXT PRIMARY KEY, seq INTEGER, actor TEXT NOT NULL,
  capability TEXT NOT NULL, target TEXT NOT NULL,
  decision TEXT NOT NULL, at_ms INTEGER NOT NULL,
  correlation_id TEXT
);

CREATE TABLE knowledge_pages (
  id TEXT PRIMARY KEY, source TEXT NOT NULL, title TEXT NOT NULL,
  url TEXT, content_hash TEXT NOT NULL, word_count INTEGER NOT NULL,
  status TEXT NOT NULL, updated_ms INTEGER NOT NULL
);
CREATE TABLE knowledge_chunks (
  id TEXT PRIMARY KEY, page_id TEXT NOT NULL, idx INTEGER NOT NULL,
  text TEXT NOT NULL, embedding BLOB
);
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
  chunk_id UNINDEXED, page_id UNINDEXED, text,
  tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE identity_aliases (alias TEXT PRIMARY KEY, canonical TEXT NOT NULL);
CREATE TABLE devices (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, token_hash TEXT NOT NULL,
  scopes TEXT NOT NULL, created_ms INTEGER NOT NULL,
  expires_ms INTEGER, last_seen_ms INTEGER, revoked_ms INTEGER
);
CREATE TABLE telemetry_metrics (
  bucket_ms INTEGER NOT NULL,
  name TEXT NOT NULL,
  attrs_hash TEXT NOT NULL,
  attrs_json TEXT NOT NULL,
  value_sum REAL NOT NULL,
  count INTEGER NOT NULL,
  last_ms INTEGER NOT NULL,
  PRIMARY KEY (bucket_ms, name, attrs_hash)
);
CREATE TABLE telemetry_spans (
  trace_id TEXT NOT NULL,
  span_id TEXT NOT NULL,
  parent_span_id TEXT,
  name TEXT NOT NULL,
  service TEXT NOT NULL,
  start_ms INTEGER NOT NULL,
  duration_ms REAL NOT NULL,
  status TEXT NOT NULL,
  attrs_json TEXT,
  PRIMARY KEY (trace_id, span_id)
);
CREATE INDEX telemetry_spans_start ON telemetry_spans(start_ms);

CREATE TABLE summaries (
  key TEXT PRIMARY KEY,
  content_hash TEXT NOT NULL,
  text TEXT NOT NULL,
  created_ms INTEGER NOT NULL
);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE projection_checkpoint (name TEXT PRIMARY KEY, seq INTEGER NOT NULL);
CREATE TABLE outbox (
  id INTEGER PRIMARY KEY AUTOINCREMENT, seq INTEGER NOT NULL,
  kind TEXT NOT NULL, payload TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0, next_attempt_ms INTEGER NOT NULL
);
CREATE TABLE _migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, checksum TEXT NOT NULL);
```

`events_archive` memakai DDL yang sama tanpa indeks sekunder. Migrasi ditulis
sebagai file SQL bernomor, di-embed dengan `include_str!`, dijalankan dalam
transaksi, dan diverifikasi checksum-nya.

## 4. Patch realtime

```rust
pub struct Patch {
    pub seq: Seq,
    pub upserts: Vec<PatchEntity>, // agent | task | alert | usage | connector | approval
    pub deletes: Vec<PatchRef>,
}
```

`PatchEntity` adalah enum bertag yang berisi bentuk API (`AgentCard`, `TaskView`,
`AlertView`, …) plus payload teredaksi. Representasi wire selalu menyertakan
field `type` (agent/task/alert/usage/connector/approval) dan data entity yang
cukup untuk menerapkan patch. Patch dibangun dari event yang baru di-append,
bukan dari perbedaan tabel. Klien menerapkan patch setelah snapshot `seq`; jika
`seq` melompat, klien meminta snapshot penuh.

## 5. API v1

Base `/api/v1`, JSON, gzip, `x-request-id`. Semua response error:

```json
{ "code": "forbidden", "message": "capability required", "detail": null, "request_id": "..." }
```

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/overview` | — | `Overview` |
| GET | `/agents` | `?status=&kind=&q=` | `Page<AgentCard>` |
| GET | `/agents/{id}` | `?as_of=<ms>` | `AgentDetail` |
| GET | `/agents/{id}/tasks` | `?phase=&limit=` | `Page<TaskView>` |
| GET | `/agents/{id}/logs` | `?level=&q=&limit=&before=` | `Page<LogView>` |
| GET | `/agents/{id}/usage` | `?window=24h|7d|30d` | `UsageReport` |
| GET | `/agents/{id}/sessions` | — | `Page<SessionView>` |
| GET | `/agents/{id}/files` | — | `Vec<ChangedFile>` |
| GET | `/tasks` | `?agent=&phase=&q=` | `Page<TaskView>` |
| GET | `/alerts` | `?state=&level=` | `Vec<AlertView>` |
| GET | `/alerts/{id}` | — | `AlertView` |
| POST | `/alerts/{id}/ack` | `{note?}` | `AlertView` |
| POST | `/alerts/{id}/snooze` | `{minutes, note?}` | `AlertView` |
| POST | `/alerts/{id}/resolve` | `{note?}` | `AlertView` |
| GET | `/timeline` | `?since=&until=&kind=&subject=&limit=` | `Page<EventView>` |
| GET | `/usage` | `?group_by=agent|model|provider&window=` | `UsageReport` |
| GET | `/costs` | `?window=` | `CostReport` |
| GET | `/markets` | — | `MarketsView` |
| POST | `/markets/refresh` | `{symbols?}` | `MarketsView` |
| GET | `/connectors` | — | `Vec<ConnectorView>` |
| POST | `/connectors/{id}/test` | — | `ConnectorTest` |
| POST | `/connectors/{id}/poll` | — | `ConnectorTest` |
| POST | `/connectors/{id}/enable` \| `/disable` | — | `ConnectorView` |
| POST | `/connectors/{id}/actions/{action}` | `{input}` | `ActionOutcome` |
| GET | `/approvals` | `?state=` | `Vec<ApprovalView>` |
| POST | `/approvals/{id}/approve` \| `/deny` | `{nonce, note?}` | `ApprovalView` |
| POST | `/approvals/{id}/ticket` | — | `{nonce, expires_ms}` |
| POST | `/actions` | `{capability, target, input, note?, dry_run?}` | `ActionOutcome` |
| GET | `/audits` | `?actor=&capability=&from=&to=&limit=` | `Page<AuditView>` |
| GET | `/config/policy` | — | `PolicyView` |
| PUT | `/config/policy` | `{yaml, confirm}` | `PolicyView` |
| GET | `/agents/{id}/commands` | `?wait=30` | `Vec<AgentCommand>` |
| POST | `/agents/{id}/commands/{cid}/result` | `{ok, output}` | `204` |
| GET | `/knowledge/search` | `?q=&limit=` | `Vec<KnowledgeHit>` |
| GET | `/knowledge/pages` | `?source=&status=&limit=` | `Page<KnowledgePageView>` |
| GET | `/knowledge/pages/{id}` | — | `KnowledgePageView` |
| POST | `/knowledge/sync` | `{connector?}` | `KnowledgeSync` |
| POST | `/knowledge/promote` | `{page_id, title}` | `KnowledgePromote` |
| POST | `/heartbeat` | `HeartbeatIn` | `HeartbeatOut` |
| POST | `/ingest/otlp/v1/metrics` \| `/traces` \| `/logs` | protobuf/JSON | `204` |
| POST | `/webhooks/{connector}` | raw + header | `204` |
| POST | `/auth/pair` | `{code, name}` | `{token, device_id, expires_ms}` |
| POST | `/auth/ws-ticket` | cookie | `{ticket, expires_ms}` |
| GET | `/ws` | `?ticket=&since=` | WebSocket |
| GET | `/events/stream` | `?since=` | SSE |
| GET | `/metrics` | — | Prometheus text |
| GET | `/settings` | — | `Settings` |
| PATCH | `/settings` | partial `Settings` | `Settings` |
| GET | `/devices` | — | `Vec<DeviceView>` |
| DELETE | `/devices/{id}` | — | `204` |
| GET | `/backup` | — | `BackupStatus` |
| POST | `/backup` | `{note?}` | `BackupStatus` |
| GET | `/openapi.json` | — | OpenAPI 3.1 |
| GET | `/health` | — | `Health` |
| GET | `/doctor` | — | `DoctorReport` |

Tipe response didefinisikan di `hq-api::dto`, derive `Serialize`, `Deserialize`,
`utoipa::ToSchema`. Klien TS digenerate dari `/openapi.json` ke
`web/src/lib/api/schema.d.ts` dan dibungkus `web/src/lib/api/client.ts`.

Auth: lihat `00-design.md` §9.5. Middleware urutan: request-id → rate-limit →
auth → CSRF (mutasi cookie) → capability (handler) → audit.

## 6. Config

| Env | Default | Arti |
|---|---|---|
| `HQ_PORT` | `8900` | port |
| `HQ_BIND` | `127.0.0.1` | alamat |
| `HQ_DATA` | XDG data | DB, model, backup |
| `HQ_CONFIG` | XDG config | config, policy, connector, secret |
| `HQ_STATE` | XDG state | pid, laporan migrate, doctor cache |
| `HQ_MASTER_KEY_FILE` | `$HQ_CONFIG/master.key` | age identity |
| `HQ_SECRETS_FILE` | `$HQ_CONFIG/secrets.age` | blob secret |
| `HQ_ALLOW_SEED` | `0` | izinkan seed/dry-run di build rilis (CI saja) |
| `HQ_INTERVAL` | `6` | detik siklus collector |
| `HQ_TRUST_LOOPBACK` | `1` | loopback = owner |
| `HQ_RETENTION_DAYS` | `90` | ambang arsip |
| `HQ_LOCALE` | `id` | locale default API |
| `HQ_TZ` | `Asia/Jakarta` | zona tampilan |
| `HQ_LOG` | `info` | level tracing |
| `HQ_OTLP` | `0` | terima OTLP |
| `HQ_TRACE_SAMPLE` | `0.1` | sampling trace |
| `HQ_EMBED` | `on` | embedding knowledge |

Config file `config.toml` opsional; env menang. Secret tidak pernah di config.

## 7. Error dan observability

- Setiap crate punya `thiserror` enum; `hq-api` memetakan ke HTTP + kode stabil.
- Span tracing: `request`, `append`, `project`, `connector.poll`,
  `connector.webhook`, `action.execute`, `alert.evaluate`, `mcp.call`.
- Metrik: `hq_events_total{kind}`, `hq_append_ms`, `hq_projection_lag_seq`,
  `hq_connector_up{id}`, `hq_connector_poll_ms`, `hq_ws_clients`,
  `hq_alerts_open{level}`, `hq_actions_total{result}`,
  `hq_approvals_pending`, `hq_notifications_total{channel,ok}`,
  `hq_connector_auth_failures_total{id}`, `hq_knowledge_index_lag_seq`,
  `hq_audit_write_ms`, `hq_backup_age_seconds`.
- `/health`: status DB, WAL size, seq terakhir, lag projection, connector down,
  secret decrypt, model embedding, disk, uptime.
- `/doctor`: cek yang sama + saran perbaikan, dipakai `hq doctor`.

## 8. Batas yang tidak boleh dilanggar

1. Tidak ada SQL di luar `hq-store`.
2. Tidak ada HTTP keluar selain di `hq-connectors` dan executor connector.
3. Tidak ada teks bebas masuk store sebelum `redact`.
4. Tidak ada aksi agent tanpa `hq-control` (capability + audit).
5. Tidak ada event tanpa `subject` dan `source`.
6. Tidak ada response API tanpa skema OpenAPI.
7. Tidak ada aset UI dari jaringan saat runtime.
8. Tidak ada `unwrap` pada input pengguna, connector, atau file.
