# 03 — Connectors

Dokumen ini menetapkan manifest, trait, supervisor, engine deklaratif, secret,
katalog, dan definisi selesai connector. Kontrak tipe inti, `EventDraft`,
`Subject`, `SourceId`, dan skema DB mengikuti
[`01-architecture.md`](01-architecture.md) §2–§3. Keputusan katalog dan secret
mengikuti [`00-design.md`](00-design.md) §11. Dokumen ini tidak mengulang
alasan desain; ia menetapkan bentuk yang dibangun.

## 1. Manifest dan trait

### 1.1 Lokasi

| Path | Isi |
|---|---|
| `crates/hq-connectors/src/manifest.rs` | `Manifest`, `ManifestMeta`, `AuthSpec`, `PollSpec`, `WebhookSpec`, `ActionSpec`, validasi |
| `crates/hq-connectors/src/lib.rs` | `Connector`, `ConnectorCtx`, `ConnectorError`, `Health`, `Tier` |
| `crates/hq-connectors/src/supervisor.rs` | `Supervisor`, `ConnectorRuntime`, `ConnectorSink` |
| `crates/hq-connectors/src/engine/` | `auth.rs`, `pagination.rs`, `mapping.rs`, `webhook.rs`, `action.rs`, `template.rs` |
| `crates/hq-connectors/src/native/` | `github.rs`, `telegram.rs`, `mcp.rs`, `otlp.rs`, `prometheus.rs`, `notion.rs`, `eodhd.rs`, `fmp.rs`, `heartbeat.rs`, `local.rs` |
| `crates/hq-connectors/manifests/` | manifest YAML Tier 2 dan Tier 3, di-embed `include_str!` |
| `crates/hq-connectors/tests/` | corpus manifest, fake server, webhook, live smoke |
| `docs/connectors/<id>.md` | dokumen auth, scope, event, action, live smoke |

`hq-connectors` tidak boleh bergantung pada `hq-store`, `hq-api`, atau
`rusqlite` (01 §1). Efek samping store masuk lewat trait `ConnectorSink` yang
di-wire `hq-server`.

### 1.2 Struct `Manifest`

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u16,
    pub meta: ManifestMeta,
    pub auth: AuthSpec,
    pub poll: Option<PollSpec>,
    pub webhook: Option<WebhookSpec>,
    pub actions: BTreeMap<String, ActionSpec>,
    pub retry: RetrySpec,
    pub rate_limit: RateLimitSpec,
    pub health: HealthSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestMeta {
    pub id: String,
    pub name: String,
    pub tier: Tier,
    pub icon: String,
    pub description: String,
    pub docs: String,
    pub event_kinds: Vec<String>,
    pub scopes: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Native,
    Preset,
    Generic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AuthSpec {
    None,
    Bearer {
        token: SecretRef,
    },
    Basic {
        username: SecretRef,
        password: SecretRef,
    },
    ApiKey {
        #[serde(rename = "in")]
        location: ApiKeyLocation,
        name: String,
        value: SecretRef,
    },
    OAuth2ClientCredentials {
        token_url: String,
        client_id: SecretRef,
        client_secret: SecretRef,
        scopes: Vec<String>,
        token_path: SecretPath,
    },
    OAuth2AuthorizationCode {
        authorization_url: String,
        token_url: String,
        redirect_uri: String,
        client_id: SecretRef,
        client_secret: SecretRef,
        scopes: Vec<String>,
        pkce: bool,
        token_path: SecretPath,
        refresh_path: Option<SecretPath>,
    },
    Native {
        kinds: Vec<String>,
        note: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PollSpec {
    pub url: String,
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub cadence_ms: u32,
    pub timeout_ms: u32,
    pub pagination: PaginationSpec,
    pub mapping: MappingSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PaginationSpec {
    None,
    Offset {
        offset_param: String,
        limit_param: String,
        limit: u32,
        start: u32,
        total_path: Option<String>,
    },
    Page {
        page_param: String,
        limit_param: Option<String>,
        limit: u32,
        start: u32,
        has_more_path: Option<String>,
    },
    Cursor {
        cursor_param: String,
        cursor_path: String,
        stop_when_empty: bool,
    },
    Link {
        header: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingSpec {
    pub events: Vec<EventMap>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventMap {
    pub event_type: String,
    pub when: Option<String>,
    pub subject: String,
    pub items: String,
    pub id: Option<String>,
    pub ts_field: Option<String>,
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, ValueMap>,
    #[serde(default)]
    pub static_fields: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub dedupe: DedupeSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueMap {
    Literal { literal: serde_json::Value },
    Expr(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DedupeSpec {
    #[serde(default)]
    pub key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookSpec {
    pub path: String,
    pub max_body_bytes: u32,
    pub signature: SignatureSpec,
    pub events: Vec<EventMap>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureSpec {
    pub algorithm: SignatureAlgorithm,
    pub secret: SecretRef,
    pub header: String,
    pub prefix: Option<String>,
    pub encoding: SignatureEncoding,
    pub timestamp_header: Option<String>,
    pub timestamp_format: TimestampFormat,
    pub max_skew_s: u32,
    pub signed_payload: String,
    pub nonce: Option<NonceSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithm {
    HmacSha256,
    HmacSha1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureEncoding {
    Hex,
    Base64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampFormat {
    UnixSeconds,
    UnixMs,
    Rfc3339,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonceSpec {
    pub source: NonceSource,
    pub header: Option<String>,
    pub field: Option<String>,
    pub ttl_s: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NonceSource {
    Header,
    Body,
    Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionSpec {
    Http(HttpAction),
    Native(NativeAction),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpAction {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub expect: ExpectSpec,
    pub timeout_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAction {
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectSpec {
    pub status: Vec<u16>,
    pub jsonpath: Option<String>,
    pub equals: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrySpec {
    pub max_attempts: u8,
    pub base_delay_ms: u32,
    pub max_delay_ms: u32,
    pub retry_on: Vec<u16>,
    pub respect_retry_after: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitSpec {
    pub requests_per_minute: u32,
    pub burst: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthSpec {
    pub probe: Option<HealthProbe>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthProbe {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub expect_status: Vec<u16>,
    pub interval_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretPath(pub String);
```

`SecretRef` hanya menerima `${secret:<path>}`. `SecretPath` hanya menerima
path `[a-z0-9][a-z0-9_.-]{1,64}`. Plaintext token di YAML adalah error
validasi, bukan pilihan.

### 1.3 Trait dan tipe pendukung

```rust
use async_trait::async_trait;
use hq_core::{EventDraft, Ms, SourceId};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32};
use std::sync::{Arc, RwLock};

#[async_trait]
pub trait Connector: Send + Sync {
    fn manifest(&self) -> &Manifest;
    async fn poll(&self, ctx: &ConnectorCtx<'_>) -> Result<Vec<EventDraft>, ConnectorError>;
    async fn handle_webhook(
        &self,
        req: WebhookRequest,
    ) -> Result<Vec<EventDraft>, ConnectorError>;
    async fn action(
        &self,
        action: &str,
        input: serde_json::Value,
    ) -> Result<ActionOutcome, ConnectorError>;
    async fn health(&self) -> Health;
}

pub struct ConnectorCtx<'a> {
    pub now_ms: Ms,
    pub deadline_ms: Ms,
    pub http: &'a reqwest::Client,
    pub secrets: Arc<dyn SecretProvider>,
    pub nonces: &'a dyn NonceStore,
    pub config: &'a ConnectorConfig,
    pub state: &'a ConnectorState,
    pub engine: Option<&'a Engine>,
    pub home: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub host: crate::runtime::HostKind,
}

pub struct ConnectorConfig {
    pub values: serde_json::Value,
}

pub struct ConnectorState {
    pub cursor: RwLock<Option<String>>,
    pub health: RwLock<Health>,
    pub last_ok_ms: AtomicI64,
    pub failures: AtomicU32,
    pub auth_failures: AtomicU32,
    pub running: AtomicBool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Health {
    pub status: HealthStatus,
    pub last_ok_ms: Option<Ms>,
    pub last_error: Option<HealthError>,
    pub next_run_ms: Option<Ms>,
    pub consecutive_failures: u32,
    pub checked_ms: Ms,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Unconfigured,
    Unknown,
    Healthy,
    Degraded,
    Down,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthError {
    pub code: String,
    pub message: String,
    pub at_ms: Ms,
}

#[derive(Clone, Debug)]
pub struct WebhookRequest {
    pub connector_id: String,
    pub method: String,
    pub headers: reqwest::header::HeaderMap,
    pub body: Vec<u8>,
    pub received_ms: Ms,
    pub remote_addr: Option<std::net::SocketAddr>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub action: String,
    pub ok: bool,
    pub status: Option<u16>,
    pub request_id: Option<String>,
    pub data: serde_json::Value,
    pub duration_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("unconfigured: {0}")]
    Unconfigured(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("auth: {0}")]
    Auth(String),
    #[error("rate limited; retry_after_ms={retry_after_ms}")]
    RateLimited { retry_after_ms: i64 },
    #[error("http status {status}")]
    Http { status: u16 },
    #[error("parse: {0}")]
    Parse(String),
    #[error("schema: {0}")]
    Schema(String),
    #[error("timeout after {0} ms")]
    Timeout(u64),
    #[error("webhook: {0}")]
    Webhook(String),
    #[error("action: {0}")]
    Action(String),
    #[error("degraded: {0}")]
    Degraded(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub trait SecretProvider: Send + Sync {
    fn get(&self, path: &str) -> Result<SecretString, SecretError>;
    fn set(&self, path: &str, value: &str) -> Result<(), SecretError>;
    fn list(&self) -> Result<Vec<SecretMeta>, SecretError>;
}

#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretString(***)")
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        // Best-effort: kosongkan buffer. Zeroize penuh ditambahkan saat S17
        // bila dependency zeroize disetujui; tidak ada salinan nilai di Debug.
        self.0.clear();
    }
}

#[derive(Clone, Debug)]
pub struct SecretMeta {
    pub path: String,
    pub fingerprint: String,
    pub updated_ms: Ms,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("secret not found: {0}")]
    NotFound(String),
    #[error("secret store unavailable")]
    Unavailable,
    #[error("decrypt failed")]
    Decrypt,
    #[error("permissions too broad: {0}")]
    Permissions(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum NonceError {
    #[error("nonce store unavailable")]
    Unavailable,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("read: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(String),
    #[error("validation failed with {0} errors")]
    Validation(usize),
}

pub trait NonceStore: Send + Sync {
    fn claim(&self, key: &str, ttl_ms: i64, now_ms: i64) -> Result<bool, NonceError>;
}

pub trait ConnectorSink: Send + Sync {
    fn record_run(&self, run: RunRecord) -> Result<(), ConnectorError>;
    fn set_next_run(&self, id: &str, next_ms: i64) -> Result<(), ConnectorError>;
    fn emit(&self, draft: EventDraft) -> Result<(), ConnectorError>;
    fn set_health(&self, id: &str, health: &Health) -> Result<(), ConnectorError>;
}
```

`EventDraft` adalah tipe `hq_core` 01 §2; connector tidak mendefinisikan ulang.
Bentuk exact yang dipakai:

```rust
// hq-core 01 §2; tidak didefinisikan ulang di hq-connectors
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
```

Pemetaan error ke API (`hq-api` memakai tabel ini):

| `ConnectorError` | HTTP | Kode body |
|---|---|---|
| `Unconfigured` | 409 | `unconfigured` |
| `Auth` | 401 | `connector_auth` |
| `RateLimited` | 429 | `rate_limited` |
| `Timeout` | 504 | `timeout` |
| `Parse` / `Schema` | 502 | `connector_bad_payload` |
| `Webhook` | 401 atau 409 | `webhook_invalid` / `webhook_replay` |
| `Action` | 502 | `action_failed` |
| `Http` | 502 | `connector_http` |
| `Degraded` | 503 | `connector_degraded` |
| `Unsupported` | 405 | `unsupported` |
| `Io` | 500 | `internal` |

Constraint connector:

- `source` wajib `SourceId("connector:<manifest.id>")`.
- `subject` hanya `Subject::Connector(<id>)`, `Subject::Agent(...)` dari
  `subject_hint` yang dideklarasikan, atau `Subject::System` untuk health.
- `idempotency_key` wajib untuk event yang berasal dari payload eksternal.
- `payload` untuk `connector.event` tepat berisi
  `{connector_id, event_type, subject_hint, data}` (00 §9.3).

### 1.4 Contoh YAML

Contoh lengkap Tier 2 `rss`; field cocok dengan struct di §1.2.

```yaml
schema_version: 1
meta:
  id: rss
  name: RSS
  tier: preset
  icon: "M4 4h16v16H4z M6 8h6a4 4 0 0 1 4 4v6 M6 16h2"
  description: "Feed RSS/Atom untuk sinyal eksternal."
  docs: docs/connectors/rss.md
  event_kinds:
    - feed.item
  scopes: []
auth:
  mode: none
poll:
  url: "${config:rss.feed_url}"
  method: GET
  headers:
    Accept: "application/rss+xml, application/atom+xml"
  query: {}
  body: null
  cadence_ms: 300000
  timeout_ms: 10000
  pagination:
    type: none
  mapping:
    events:
      - event_type: feed.item
        subject: "connector:rss"
        items: "$.items[*]"
        id: "$.guid"
        ts_field: "$.pubDate"
        fields:
          title: "$.title"
          link: "$.link"
          published: "$.pubDate"
        static_fields: {}
        dedupe:
          key: "${field.link}"
webhook: null
actions: {}
retry:
  max_attempts: 3
  base_delay_ms: 1000
  max_delay_ms: 30000
  retry_on: [429, 500, 502, 503, 504]
  respect_retry_after: true
rate_limit:
  requests_per_minute: 12
  burst: 2
health:
  probe: null
```

Tidak ada `...` atau field opsional yang dihilangkan. `rss` menjadi
`unconfigured` sampai `config.rss.feed_url` diisi.

### 1.5 Tier

| Nilai YAML | Arti | Bentuk |
|---|---|---|
| `native` | Rust code di `hq-connectors`; manifest mendokumentasikan auth/event/action | Rust |
| `preset` | implementasi penuh dari engine deklaratif + manifest YAML | YAML |
| `generic` | engine yang sama, manifest dibuat pengguna | YAML |

Tier bukan label kosmetik: ia menentukan siapa yang boleh mengubah perilaku.
Tier `native` hanya berubah lewat kode + review; `preset` dan `generic` berubah
lewat YAML + validasi.

## 2. Supervisor

### 2.1 Model task

Satu task `tokio` per connector aktif. Supervisor menyimpan
`ConnectorRuntime`:

```rust
pub struct ConnectorRuntime {
    pub manifest: Manifest,
    pub connector: Arc<dyn Connector>,
    pub state: Arc<ConnectorState>,
    pub next_run_ms: AtomicI64,
}

pub struct RunRecord {
    pub id: String,
    pub connector_id: String,
    pub started_ms: i64,
    pub finished_ms: i64,
    pub ok: bool,
    pub events: usize,
    pub duplicates: usize,
    pub error: Option<String>,
}

pub struct RunReport {
    pub events: usize,
    pub duplicates: usize,
    pub duration_ms: u64,
    pub health: Health,
}
```

`Supervisor::run(shutdown: watch::Receiver<bool>)`:

1. Muat `next_run_ms` dari tabel `connectors` (01 §3); connector yang belum
   punya baris dianggap due sekarang.
2. Untuk setiap runtime, tunggu sampai `max(now, next_run_ms)`.
3. `state.running.compare_exchange(false, true, AcqRel, Acquire)`; jika gagal,
   lewati tick, catat metrik `hq_connector_skipped_total{id}`, dan jangan
   antrekan run kedua. Tidak ada run yang tumpang tindih.
4. Panggil `connector.poll(&ctx)` dengan `deadline_ms = now + timeout_ms`;
   bungkus `tokio::time::timeout`.
5. Kirim draft lewat `ConnectorSink::emit`; store yang melakukan append +
   projection + idempotency. Duplikat dihitung dari `append` yang no-op.
6. Tulis `connector_runs` + `next_run_ms` + health; lepaskan `running`.
7. Ulangi sampai `shutdown` berubah; run yang sedang berjalan diberi 5 detik
   untuk selesai, lalu task di-abort.

Webhook-only connector tidak punya poll periodik; ia tetap punya task untuk
health sweep `health.probe.interval_ms` dan re-evaluasi secret.

### 2.2 Backoff dan jitter

```text
base = max(poll.cadence_ms, 15_000)
exp = 1 << min(consecutive_failures, 6)
raw = min(base * exp, 900_000)
jitter_permille = (hash(connector_id, failures) % 401) - 200   // -20%..+20%
delay_ms = raw * (1000 + jitter_permille) / 1000
next_run_ms = now_ms + max(delay_ms, 1000)
```

`hash` memakai `std::collections::hash_map::DefaultHasher` atas
`(connector_id, failures)`; tidak ada dependency acak baru. Jitter mencegah
semua connector jatuh bersamaan setelah restart. `Retry-After` pada 429
mengalahkan backoff: `next_run_ms = now + retry_after`.

### 2.3 Circuit breaker

| Parameter | Nilai |
|---|---|
| Buka setelah gagal non-auth berturut-turut | 5 |
| Buka setelah gagal auth berturut-turut | 3 |
| Half-open setelah | 60.000 ms |
| Auth cooldown sebelum mencoba kredensial yang sama | 900.000 ms |
| Tutup setelah | 1 poll sukses |

Saat open: poll dilewati, health `down`, `next_run_ms` = waktu half-open.
Saat half-open: tepat satu percobaan; sukses → `healthy` dan reset counter;
gagal → `down` + backoff. `connector.auth_failed` dipancarkan sekali per
episode auth, bukan setiap percobaan.

### 2.4 Rate limit

Token bucket per connector:

- kapasitas `rate_limit.burst`;
- refill `rate_limit.requests_per_minute / 60` token per detik;
- sebelum request, `try_take(1)`; jika kosong, `next_run_ms = now +
  ceil((1 - tokens) / refill_per_ms)`;
- 429 dengan `Retry-After` menimpa `next_run_ms`;
- 429 tanpa header memakai exponential backoff §2.2.

Rate limit berlaku untuk poll, health probe, dan action HTTP. Webhook ingress
memakai rate limit API global, bukan bucket connector.

### 2.5 Transisi health

| Dari | Pemicu | Ke | Event |
|---|---|---|---|
| `unconfigured` | semua secret wajib ada + enabled | `unknown` | `connector.health_changed` |
| `unknown` | poll/health probe pertama sukses | `healthy` | `connector.health_changed` |
| `healthy` | gagal transien pertama | `degraded` | `connector.health_changed` |
| `degraded` | gagal ke-5 atau circuit open | `down` | `connector.health_changed` |
| `degraded` | sukses | `healthy` | `connector.health_changed` |
| `down` | half-open sukses | `healthy` | `connector.health_changed` |
| `down` | auth gagal ke-3 | `down` | `connector.auth_failed` |
| any | secret wajib hilang | `unconfigured` | `connector.health_changed` |
| any | owner disable | `disabled` | `connector.health_changed` |
| `disabled` | owner enable | `unknown` | `connector.health_changed` |

`Health.last_error` hanya `{code, message, at_ms}` yang sudah di-redact.
`code` stabil: `auth_failed`, `rate_limited`, `http_5xx`, `timeout`,
`parse_error`, `schema_error`, `webhook_invalid`, `unconfigured`,
`adapter_partial_failure`.

### 2.6 Event supervisor

| Event | Kapan | Payload |
|---|---|---|
| `connector.registered` | boot pertama atau hash manifest berubah | `{connector_id, tier, manifest_hash}` |
| `connector.health_changed` | status berubah | `{connector_id, old, new, code, at_ms}` |
| `connector.poll_completed` | events > 0, duplicates > 0, atau gagal; maksimal 1 per 60 detik per connector | `{connector_id, ok, events, duplicates, duration_ms}` |
| `connector.auth_failed` | 401/403 pertama per episode | `{connector_id, status, retry_at_ms}` |
| `connector.event` | engine memetakan payload | `{connector_id, event_type, subject_hint, data}` |

`connector.poll_completed` yang sukses tanpa event tidak dipancarkan; cukup
`connector_runs`. Ini menjaga volume < 20.000/hari (02 §4.5).

### 2.7 Tanpa overlap

- `state.running` adalah `AtomicBool`; `compare_exchange` mencegah dua run.
- `connector_runs` hanya dibuat setelah `running` didapat; run yang dilewati
  tidak menulis baris.
- Action dan webhook tidak memakai `running` poll; keduanya punya rate limit
  sendiri dan tidak boleh memblokir poll.
- Jika run melewati deadline, `timeout` membatalkan future; child HTTP/command
  harus dibatalkan oleh connector. Run dicatat `ok = 0`, `error = "timeout"`.

## 3. Engine deklaratif

### 3.1 Schema YAML

`schema_version` wajib `1`. Semua field tanpa `default` wajib.

| Bagian | Field | Tipe | Aturan |
|---|---|---|---|
| root | `schema_version` | u16 | harus `1` |
| root | `meta` | object | §1.2 |
| root | `auth` | object | §3.2 |
| root | `poll` | object/null | wajib bila connector punya poll |
| root | `webhook` | object/null | wajib bila connector menerima webhook |
| root | `actions` | map | nama aksi `[a-z][a-z0-9_]{1,40}` |
| root | `retry` | object | §3.6 |
| root | `rate_limit` | object | §2.4 |
| root | `health` | object | `probe` boleh null |
| `meta` | `id` | string | `[a-z0-9][a-z0-9-]{1,47}`, unik |
| `meta` | `name` | string | 1–64 karakter |
| `meta` | `tier` | enum | `native`, `preset`, `generic` |
| `meta` | `icon` | string | SVG path `d`, maksimal 4 KiB |
| `meta` | `docs` | string | harus `docs/connectors/<id>.md` |
| `meta` | `event_kinds` | array | `[a-z][a-z0-9_.-]{2,64}`; untuk `connector.event` |
| `meta` | `scopes` | array | string; dokumentasi scope OAuth/API |
| `poll` | `url` | string | `http`/`https`; template diperbolehkan |
| `poll` | `method` | enum | `GET`, `POST`, `PUT`, `PATCH`, `DELETE` |
| `poll` | `headers`/`query` | map | nilai template; secret diperbolehkan |
| `poll` | `body` | JSON/null | template di dalam string |
| `poll` | `cadence_ms` | u32 | 15.000–86.400.000 |
| `poll` | `timeout_ms` | u32 | 1.000–60.000 |
| `poll` | `pagination` | object | §3.3 |
| `poll` | `mapping` | object | minimal 1 event |
| `mapping.events[]` | `event_type` | string | harus ada di `meta.event_kinds` |
| `mapping.events[]` | `when` | string/null | ekspresi §3.4 |
| `mapping.events[]` | `subject` | string | `connector:<id>` atau `agent:<slug>` |
| `mapping.events[]` | `items` | JSONPath | subset §3.3 |
| `mapping.events[]` | `id` | JSONPath/null | untuk dedupe |
| `mapping.events[]` | `ts_field` | JSONPath/template/null | default `now_ms` |
| `mapping.events[]` | `required_fields` | array | nama di `fields` |
| `mapping.events[]` | `fields` | map | JSONPath, `{literal: ...}`, atau template |
| `mapping.events[]` | `static_fields` | map | nilai JSON literal |
| `mapping.events[]` | `dedupe.key` | template | default hash data |
| `webhook` | `path` | string | harus `/webhooks/<id>` |
| `webhook` | `max_body_bytes` | u32 | 1.024–1.048.576; default 1.048.576 |
| `webhook` | `signature` | object | §3.5; wajib |
| `webhook` | `events` | array | sama dengan mapping |
| `signature` | `algorithm` | enum | `hmac_sha256`, `hmac_sha1` |
| `signature` | `secret` | SecretRef | `${secret:...}` |
| `signature` | `header` | string | nama header signature |
| `signature` | `prefix` | string/null | mis. `v0=` atau `sha256=` |
| `signature` | `encoding` | enum | `hex`, `base64` |
| `signature` | `timestamp_header` | string/null | wajib bila tidak ada nonce |
| `signature` | `timestamp_format` | enum | `unix_seconds`, `unix_ms`, `rfc3339` |
| `signature` | `max_skew_s` | u32 | 30–3.600 |
| `signature` | `signed_payload` | string | template `{body}`, `{timestamp}`, `{nonce}` |
| `signature` | `nonce` | object/null | §3.5 |
| `actions.<name>` | `type` | enum | `http`, `native` |
| `actions.<name>.type=http` | `method`,`url`,`headers`,`query`,`body`,`expect`,`timeout_ms` | — | §3.6 |
| `actions.<name>.type=native` | `description` | string | implementasi Rust |
| `retry` | lihat §3.6 | — | — |
| `rate_limit` | `requests_per_minute`,`burst` | u32 | keduanya > 0 |
| `health.probe` | `method`,`url`,`headers`,`expect_status`,`interval_ms` | — | `interval_ms` 30.000–3.600.000 |

Batas ukuran file manifest: 256 KiB. Parser memakai `serde_yaml` dengan
`deny_unknown_fields`; YAML anchor/alias diperbolehkan tetapi tidak boleh
membentuk siklus.

### 3.2 Mode auth

| Mode | Field wajib | Perolehan token | Penyimpanan |
|---|---|---|---|
| `none` | — | — | — |
| `bearer` | `token` | dibaca dari secrets | `${secret:...}` |
| `basic` | `username`, `password` | base64 `username:password` | dua secret |
| `api_key` | `in`, `name`, `value` | header/query | `${secret:...}` |
| `oauth2_client_credentials` | `token_url`, `client_id`, `client_secret`, `scopes`, `token_path` | `POST token_url` grant `client_credentials` | access token di `token_path` |
| `oauth2_authorization_code` | `authorization_url`, `token_url`, `redirect_uri`, `client_id`, `client_secret`, `scopes`, `pkce`, `token_path`, `refresh_path` | code exchange atau refresh token | access + refresh di secret path |

Aturan token:

- Access token di-cache di memori dengan `expires_at_ms`; refresh 60 detik
  sebelum kedaluwarsa.
- 401 pertama: buang token, refresh sekali, ulangi request satu kali.
- 401 kedua: `ConnectorError::Auth`; supervisor menaikkan `auth_failures`.
- Refresh token hanya disimpan di `refresh_path`; tidak pernah di event/log.
- `oauth2_authorization_code` menyediakan dua action bawaan yang dipanggil
  lewat route yang sudah ada `POST /api/v1/connectors/:id/actions/:action`:
  `oauth_authorize` (mengembalikan `{url, state, expires_ms}`) dan
  `oauth_exchange` (input `{code, state}`; menukar dan menyimpan token).
  State nonce 10 menit, sekali pakai. Redirect UX (halaman yang menampilkan
  `code`) adalah pilihan builder; kontrak token dan state sudah tetap.
- Native connector boleh memakai `AuthSpec::Native { kinds, note }`; kinds
  adalah dokumentasi, bukan mesin auth.

### 3.3 Pagination dan JSONPath

| Pagination | Field | Perilaku |
|---|---|---|
| `none` | — | satu request |
| `offset` | `offset_param`, `limit_param`, `limit`, `start`, `total_path` | naikkan offset; berhenti bila item < limit atau total tercapai |
| `page` | `page_param`, `limit_param`, `limit`, `start`, `has_more_path` | naikkan page; berhenti bila `has_more` false |
| `cursor` | `cursor_param`, `cursor_path`, `stop_when_empty` | cursor dari response; berhenti bila kosong/null |
| `link` | `header` | parse RFC 5988 `rel="next"` |

Batas: maksimal 20 halaman per poll; maksimal 1.000 event per poll. Jika
tercapai, cursor disimpan dan poll berikutnya melanjutkan; metrik
`hq_connector_truncated_total{id}` dinaikkan.

JSONPath yang didukung: `$`, `.field`, `['field']`, `[*]`, `[index]`.
`..`, filter `[?(...)]`, fungsi, dan script **ditolak** saat validasi. Setiap
path dikompilasi dengan `jsonpath-rust`; hasil maksimal 1.000 item. Path yang
tidak cocok menghasilkan array kosong, bukan error, kecuali `required_fields`
yang hilang.

### 3.4 Mapping dan template

Urutan evaluasi satu `EventMap`:

1. `items` menghasilkan array/objek/scalar.
2. `when` dievaluasi per item; jika false, item dilewati.
3. `fields` dievaluasi; `required_fields` yang hilang → item dilewati dan
   dihitung sebagai `mapped_skipped`.
4. `static_fields` ditambahkan.
5. `ts_field` menentukan `event_ts_ms`; gagal parse → `now_ms` dan note.
6. `dedupe.key` dievaluasi; kosong → `sha256(canonical_json(data))[..16]`.
7. `EventDraft` dibuat dengan `kind = ConnectorEvent`,
   `subject = Subject::Connector(<id>)` atau `Subject::Agent(<slug>)`,
   `source = SourceId("connector:<id>")`,
   `idempotency_key = Some("<id>:<event_type>:<dedupe>")`.

Template:

- `${secret:path}` — hanya di auth, poll header/query/body, webhook signature,
  action, dan health probe; tidak pernah di `event_type`, `subject`, `meta`.
- `${config:path}` — nilai non-secret dari `[connectors.<id>]`; `path` relatif
  terhadap section connector, mis. `slack.channel_id`.
- `${input:path}` — hanya di action.
- `${auth.token}` — di poll header/query/body, action, dan health probe.
- `${field:name}` — hanya di mapping/dedupe; mencakup `fields` dan
  `static_fields`.
- `${meta.id}` — di mana saja.
- `${env:NAME}` — hanya `poll.url` dan `health.probe.url`; nilai di-redact.
- `$${` adalah escape literal `${`.
- Jika seluruh string adalah satu referensi, tipe JSON dipertahankan; jika
  bercampur teks, hasilnya string.
- Referensi tidak dikenal/tertutup → validation error; tidak ada evaluasi
  ekspresi arbitrer.

`when` hanya mendukung grammar:

```text
expr    := term (("&&" | "||") term)*
term    := path op value | "exists(" path ")" | "(" expr ")"
path    := "$" ("." name | "[" index "]")*
op      := "==" | "!=" | ">" | ">=" | "<" | "<=" | "in"
value   := string | number | true | false | null | "[" list "]"
```

Tidak ada pemanggilan fungsi selain `exists`. Perbandingan string case-sensitive.

### 3.5 Webhook

Urutan `handle_webhook`:

1. Tolak `body.len() > max_body_bytes` → 413.
2. Baca header signature; hilangkan `prefix`; decode hex/base64; jika gagal →
   401.
3. Hitung HMAC atas `signed_payload` dengan `{body}` = body mentah,
   `{timestamp}` = header timestamp (string), `{nonce}` = nonce (string).
   Bandingkan constant-time (`hmac::Mac::verify_slice`).
4. Jika `timestamp_header` ada: parse `timestamp_format`; tolak selisih
   `abs(now - ts) > max_skew_s` → 401. Jika tidak ada timestamp, `nonce` wajib.
5. Jika `nonce` ada: ambil dari header/body/signature sesuai `source`; klaim
   `nonce:<connector>:<nonce>` dengan TTL `ttl_s`; jika sudah ada → 409.
   Nonce baru disimpan setelah signature valid.
6. Parse JSON; jika bukan JSON → 400.
7. Jalankan mapping; event yang sama (dedupe) → 204 no-op dan
   `connector_runs.duplicates++`; replay nonce/timestamp → 409.
8. Kembalikan 204. Body respons kosong.

`SignatureSpec.signed_payload` untuk Slack: `v0:{timestamp}:{body}`.
Untuk GitHub native: `{body}` + nonce `X-GitHub-Delivery`.
Webhook yang tidak punya timestamp dan tidak punya nonce ditolak validasi.

### 3.6 Actions dan retry

`HttpAction`:

- Template dievaluasi terhadap `input` + `auth` + `config`; `input` wajib objek.
- Body/response maksimal 1 MiB; response di-redact sebelum `ActionOutcome`.
- `expect.status` wajib; `expect.jsonpath`/`equals` opsional.
- `timeout_ms` 1.000–60.000.
- Retry mengikuti `RetrySpec`; hanya status di `retry_on`, error koneksi, dan
  429 (dengan `Retry-After` bila `respect_retry_after`).
- `max_attempts` 1–5; `base_delay_ms` 100–10.000; `max_delay_ms` ≥
  `base_delay_ms`, ≤ 60.000.
- Action tidak pernah menjalankan shell. Capability
  `Capability::ConnectorAction("<id>.<action>")` diperiksa `hq-control` sebelum
  engine dipanggil; connector tidak melakukan audit sendiri.

### 3.7 Kode error validasi

`validate_manifest` mengembalikan `Vec<ValidationError { code, path, message }>`.
Kode stabil:

| Kode | Pemicu |
|---|---|
| `manifest.yaml` | YAML tidak bisa diparse |
| `manifest.schema` | field salah tipe/unknown |
| `manifest.schema_version` | bukan `1` |
| `manifest.too_large` | > 256 KiB |
| `meta.id_invalid` | id tidak cocok regex |
| `meta.id_duplicate` | id sudah terdaftar |
| `meta.tier_invalid` | tier di luar enum |
| `meta.icon_invalid` | bukan SVG path |
| `meta.docs_missing` | path dokumen salah |
| `meta.event_kind_invalid` | event kind tidak cocok regex |
| `auth.mode_unsupported` | mode di luar §3.2 |
| `auth.secret_ref_invalid` | bukan `${secret:...}` |
| `auth.oauth2_field_missing` | field OAuth2 kurang |
| `auth.redirect_uri_invalid` | bukan URL absolut |
| `poll.url_invalid` | bukan http/https |
| `poll.method_invalid` | method di luar enum |
| `poll.cadence_out_of_range` | di luar 15.000–86.400.000 |
| `poll.timeout_out_of_range` | di luar 1.000–60.000 |
| `poll.pagination_unsupported` | tipe tidak dikenal |
| `poll.mapping_empty` | tidak ada event |
| `mapping.path_invalid` | JSONPath di luar subset |
| `mapping.event_type_unknown` | tidak ada di `meta.event_kinds` |
| `mapping.subject_invalid` | bukan `connector:`/`agent:` |
| `mapping.dedupe_template_invalid` | template tidak valid |
| `mapping.required_field_unknown` | nama tidak ada di `fields` |
| `webhook.path_invalid` | bukan `/webhooks/<id>` |
| `webhook.signature_missing` | webhook tanpa signature |
| `webhook.algorithm_unsupported` | algoritma tidak didukung |
| `webhook.timestamp_missing` | tanpa timestamp dan tanpa nonce |
| `webhook.nonce_invalid` | source/header/field tidak lengkap |
| `webhook.max_body_out_of_range` | di luar 1.024–1.048.576 |
| `action.name_invalid` | nama aksi tidak cocok |
| `action.url_invalid` | URL aksi tidak valid |
| `action.template_unclosed` | `${` tanpa `}` |
| `action.expect_invalid` | status/jsonpath salah |
| `retry.bounds_invalid` | batas retry salah |
| `rate_limit.invalid` | rpm/burst ≤ 0 |
| `health.probe_invalid` | probe tidak lengkap |
| `secret.plaintext_forbidden` | token terlihat di YAML |
| `secret.ref_invalid` | path secret tidak valid |

Error validasi tampil di `GET /connectors` (manifest status) dan
`POST /connectors/:id/test`; connector invalid tidak didaftarkan ke supervisor.

### 3.8 Manifest Slack lengkap

Slack memakai bot token (bearer) dan signing secret; ini jalur nyata yang tidak
memerlukan OAuth UI. OAuth2 authorization-code tetap didukung engine untuk
connector lain (Google Calendar, Teams) lewat §3.2.

```yaml
schema_version: 1
meta:
  id: slack
  name: Slack
  tier: preset
  icon: "M6 15a2 2 0 1 1-2-2h2v2z M7 15a2 2 0 0 1 4 0v5a2 2 0 1 1-4 0v-5z M9 6a2 2 0 1 1 2-2v2H9z M9 7a2 2 0 0 1 0 4H4a2 2 0 1 1 0-4h5z M18 9a2 2 0 1 1 2 2h-2V9z M17 9a2 2 0 0 1-4 0V4a2 2 0 1 1 4 0v5z M15 18a2 2 0 1 1-2 2v-2h2z M15 17a2 2 0 0 1 0-4h5a2 2 0 1 1 0 4h-5z"
  description: "Pesan, mention, dan reaksi Slack."
  docs: docs/connectors/slack.md
  event_kinds:
    - message.received
    - app_mention
    - reaction.added
  scopes:
    - channels:history
    - chat:write
    - reactions:read
    - app_mentions:read
auth:
  mode: bearer
  token: ${secret:connector.slack.bot_token}
poll:
  url: https://slack.com/api/conversations.history
  method: GET
  headers:
    Authorization: "Bearer ${auth.token}"
  query:
    channel: "${config:slack.channel_id}"
    limit: "200"
  body: null
  cadence_ms: 60000
  timeout_ms: 10000
  pagination:
    type: cursor
    cursor_param: cursor
    cursor_path: "$.response_metadata.next_cursor"
    stop_when_empty: true
  mapping:
    events:
      - event_type: message.received
        when: "exists($.messages)"
        subject: "connector:slack"
        items: "$.messages[*]"
        id: "$.ts"
        ts_field: "$.ts"
        fields:
          ts: "$.ts"
          user: "$.user"
          text: "$.text"
        static_fields:
          channel: "${config:slack.channel_id}"
        dedupe:
          key: "${field:channel}:${field:ts}"
webhook:
  path: /webhooks/slack
  max_body_bytes: 1048576
  signature:
    algorithm: hmac_sha256
    secret: ${secret:connector.slack.signing_secret}
    header: X-Slack-Signature
    prefix: "v0="
    encoding: hex
    timestamp_header: X-Slack-Request-Timestamp
    timestamp_format: unix_seconds
    max_skew_s: 300
    signed_payload: "v0:{timestamp}:{body}"
    nonce: null
  events:
    - event_type: message.received
      when: '$.type == "event_callback" && $.event.type == "message"'
      subject: "connector:slack"
      items: "$.event"
      id: "$.event_id"
      ts_field: "$.ts"
      fields:
        ts: "$.ts"
        user: "$.user"
        text: "$.text"
        channel: "$.channel"
      dedupe:
        key: "${field:channel}:${field:ts}"
    - event_type: app_mention
      when: '$.type == "event_callback" && $.event.type == "app_mention"'
      subject: "connector:slack"
      items: "$.event"
      id: "$.event_id"
      ts_field: "$.ts"
      fields:
        ts: "$.ts"
        user: "$.user"
        text: "$.text"
        channel: "$.channel"
      dedupe:
        key: "${field:channel}:${field:ts}"
actions:
  send_message:
    type: http
    method: POST
    url: https://slack.com/api/chat.postMessage
    headers:
      Authorization: "Bearer ${auth.token}"
      Content-Type: application/json
    query: {}
    body:
      channel: "${input:channel}"
      text: "${input:text}"
    expect:
      status: [200]
      jsonpath: "$.ok"
      equals: true
    timeout_ms: 10000
  update_message:
    type: http
    method: POST
    url: https://slack.com/api/chat.update
    headers:
      Authorization: "Bearer ${auth.token}"
      Content-Type: application/json
    query: {}
    body:
      channel: "${input:channel}"
      ts: "${input:ts}"
      text: "${input:text}"
    expect:
      status: [200]
      jsonpath: "$.ok"
      equals: true
    timeout_ms: 10000
  add_reaction:
    type: http
    method: POST
    url: https://slack.com/api/reactions.add
    headers:
      Authorization: "Bearer ${auth.token}"
      Content-Type: application/json
    query: {}
    body:
      channel: "${input:channel}"
      timestamp: "${input:ts}"
      name: "${input:name}"
    expect:
      status: [200]
      jsonpath: "$.ok"
      equals: true
    timeout_ms: 10000
retry:
  max_attempts: 3
  base_delay_ms: 500
  max_delay_ms: 30000
  retry_on: [429, 500, 502, 503, 504]
  respect_retry_after: true
rate_limit:
  requests_per_minute: 50
  burst: 10
health:
  probe:
    method: GET
    url: https://slack.com/api/auth.test
    headers:
      Authorization: "Bearer ${auth.token}"
    expect_status: [200]
    interval_ms: 300000
```

`nonce: null` disengaja: Slack tidak mengirim nonce; replay dicegah timestamp
(`max_skew_s`) + dedupe `event_id`/`channel:ts`, yang menghasilkan 204 no-op.
Connector yang punya header nonce (mis. `X-Webhook-Nonce`) wajib mengisi
`nonce`.

## 4. Secret

### 4.1 Path dan izin

| Path | Isi | Izin |
|---|---|---|
| `$HQ_CONFIG/secrets.age` | map YAML `path: value` terenkripsi age | 0600 |
| `$HQ_CONFIG/age.key` | age X25519 identity `AGE-SECRET-KEY-1...` | 0600 |
| `$HQ_CONFIG/secrets.age.bak-<ts>` | backup rotasi | 0600 |
| `$HQ_CONFIG/` | direktori config | 0700 |
| `$HQ_DATA/connectors/<id>/` | state non-secret (cursor, token cache metadata) | 0700 |

`HQ_CONFIG` default `~/.config/rizal-agent-hq`; `HQ_DATA` default
`~/.local/share/rizal-agent-hq`. `HQ_SECRET_KEY` (identity string) dan
`HQ_SECRET_KEY_FILE` adalah override CI; tidak pernah ditulis ke disk oleh HQ
dan tidak pernah muncul di log. File dengan bit group/other (mode `0644`)
ditolak saat load, bukan diperbaiki diam-diam.

### 4.2 Enkripsi dan master key

- Format plaintext: YAML map `path: value`; value string; path mengikuti
  `[a-z0-9][a-z0-9_.-]{1,64}`. Namespace wajib: `connector.<id>.*`,
  `agent.<slug>.*`, `system.*`.
- Enkripsi: age X25519 + XChaCha20-Poly1305; satu file untuk semua secret.
- Generate master key: `hq secret init` membuat `age.key` 0600 dengan
  `age::x25519::Identity::generate()`, menulis `AGE-SECRET-KEY-1...`, lalu
  membuat `secrets.age` kosong.
- Tulis atomik: `secrets.age.tmp` 0600 → fsync → rename; `age.key` tidak
  pernah ditulis ulang kecuali `rotate`.
- Dekripsi gagal → `SecretError::Decrypt`, health connector `unconfigured`,
  `hq doctor` menampilkan perbaikan; tidak ada fallback plaintext.

### 4.3 CLI

```bash
hq secret init
hq secret set connector.slack.bot_token --stdin
hq secret set connector.slack.bot_token --from-env SLACK_BOT_TOKEN
hq secret set connector.google.refresh_token --stdin
hq secret list
hq secret rm connector.slack.bot_token
hq secret rotate
```

| Perintah | Perilaku |
|---|---|
| `init` | buat `age.key` + `secrets.age`; gagal bila sudah ada kecuali `--force` |
| `set <path>` | baca nilai dari stdin atau `--from-env`; **tidak** menerima nilai di argv |
| `list` | cetak path + fingerprint `sha256(value)[..8]` + `updated_ms`; tidak pernah nilai |
| `rm <path>` | hapus satu path; emit `config.changed` dengan path hash |
| `rotate` | kunci baru, dekripsi semua, enkripsi ulang, verifikasi, swap; emit `secret.rotated` |

`set` menolak nilai kosong, newline di tengah path, dan path di luar namespace.
`list` butuh master key untuk fingerprint.

### 4.4 Rotasi

1. Generate identity baru.
2. Dekripsi `secrets.age` dengan kunci lama.
3. Enkripsi ulang dengan kunci baru ke `secrets.age.new`.
4. Verifikasi dekripsi `secrets.age.new` dan bandingkan map.
5. Rename `secrets.age` → `secrets.age.bak-<unix_ms>`; rename
   `secrets.age.new` → `secrets.age`; tulis `age.key` baru setelah verifikasi.
6. Simpan kunci lama sebagai `age.key.old` 0600; hapus setelah 7 hari atau
   rotasi berikutnya.
7. Emit `secret.rotated` dengan `{key_id, paths: [sha256(path)[..8]]}`; tidak
   ada nilai, tidak ada fingerprint nilai.

Jika langkah 4 gagal, file lama tidak disentuh dan rotasi gagal. Jika proses
crash setelah rename, `hq doctor` mendeteksi `age.key.old` dan meminta
`hq secret rotate --recover`.

### 4.5 Redaksi

Redaksi berjalan **sebelum** persist, log, event, error, API, WS, FTS, dan
`connector_runs.error`. Aturan:

- `hq_core::redact` adalah satu-satunya implementasi; connector tidak menulis
  pola sendiri.
- Kategori: API key (`sk-`, `ghp_`, `github_pat_`, `xox[baprs]-`, `AKIA`,
  `ASIA`, `AIza`), JWT (`eyJ...`), private key PEM, password/secret/token
  assignment, connection string `scheme://user:pass@host`, high-entropy
  ≥ 32 karakter dengan ≥ 4,0 bit/char.
- Jangan redact: git SHA-1/SHA-256 hex, UUID, ULID, timestamp, hash yang
  berlabel `sha256:`/`sha1:`.
- Redaksi idempoten; hasil redaksi tidak boleh memotong ID yang dipakai
  dedupe.
- `SecretRef`/`SecretPath` boleh muncul di error validasi (itu path, bukan
  nilai); nilai secret tidak pernah.

### 4.6 Yang dilarang

Dilarang muncul di log, event, error, `connector_runs`, `Health.last_error`,
tracing span, panic message, API/WS, FTS, dan cache:

- nilai secret dari `secrets.age`;
- header `Authorization`, `Proxy-Authorization`, `Cookie`, `Set-Cookie`,
  `X-Api-Key`, `X-Auth-Token`;
- query parameter bernama `token`, `key`, `secret`, `password`, `signature`;
- body request/response auth (`oauth/token`, login);
- access/refresh token, OAuth code, password, private key;
- argv proses yang memuat nilai secret.

Yang boleh: nama path secret (`connector.slack.bot_token`), fingerprint
`sha256(value)[..8]`, kode error, status HTTP, dan ID publik.

### 4.7 Corpus uji

| Path | Isi |
|---|---|
| `crates/hq-core/tests/corpus/redaction/api_key.txt` | minimal 3 positif + 2 negatif |
| `crates/hq-core/tests/corpus/redaction/jwt.txt` | JWT valid/expired + false positive |
| `crates/hq-core/tests/corpus/redaction/private_key.txt` | PEM multiline |
| `crates/hq-core/tests/corpus/redaction/password.txt` | assignment password/secret |
| `crates/hq-core/tests/corpus/redaction/connection_string.txt` | URL dengan user:pass |
| `crates/hq-core/tests/corpus/redaction/high_entropy.txt` | acak 32–128 char |
| `crates/hq-core/tests/corpus/redaction/false_positive_git_hash.txt` | SHA-1/SHA-256 |
| `crates/hq-core/tests/corpus/redaction/false_positive_uuid.txt` | UUID/ULID |

Tes wajib: `redacts_secret_corpus`, `api_never_returns_secret`,
`connector_error_never_contains_secret`, `manifest_rejects_plaintext_secret`,
`secret_file_permissions_enforced`, `secret_rotate_emits_no_value`.
Corpus adalah kontrak; perubahan output yang disengaja memperbarui corpus +
changelog.

## 5. Katalog

### 5.1 Konvensi

- Fixture default: `crates/hq-connectors/tests/fixtures/<id>/success.json`,
  `unauthorized.json`, `malformed.json`; webhook menambah
  `webhook-valid.json`, `webhook-invalid-signature.json`,
  `webhook-replayed.json`, `webhook-duplicate.json`.
- Live flag: `HQ_LIVE_<ID_UPPER_SNAKE>=1`; tanpa flag, tes live di-skip
  (`#[ignore]`).
- `bentuk` adalah kejujuran implementasi: `Rust` = native code; `YAML` = engine
  deklaratif; `Rust + YAML` = native client dengan config per server.
- Status tanpa kredensial selalu `unconfigured`; tidak ada data contoh.

### 5.2 Tier 1 native

| id | bentuk | auth | event / action | kredensial | fixture | live |
|---|---|---|---|---|---|---|
| `github` | Rust | GitHub App JWT + installation token, atau PAT | events `pr.opened`, `pr.merged`, `pr.closed`, `review.requested`, `issue.opened`, `issue.closed`, `check.failed`, `workflow.failed`, `release.published`; actions `comment_pr`, `create_issue`, `dispatch_workflow` | `GITHUB_TOKEN`, `GH_APP_ID`, `GH_APP_INSTALLATION_ID`, `${secret:connector.github.app_private_key}`, `GITHUB_WEBHOOK_SECRET` | `fixtures/github/` | `HQ_LIVE_GITHUB=1` |
| `telegram` | Rust | bot token + webhook secret token | events `command.received`, `callback.received`; actions `send_message`, `edit_message`, `answer_callback` | `TELEGRAM_BOT_TOKEN`, `${secret:connector.telegram.bot_token}`, `TELEGRAM_WEBHOOK_SECRET` | `fixtures/telegram/` | `HQ_LIVE_TELEGRAM=1` |
| `mcp` | Rust + YAML | stdio config atau HTTP bearer | events `mcp.tools_observed`, `mcp.tool_called`; actions `list_tools`, `call_tool` | config per server; `MCP_HTTP_TOKEN` | `fixtures/mcp/` | `HQ_LIVE_MCP=1` |
| `otlp` | Rust | none atau bearer | events `telemetry.metric`, `telemetry.trace`, `telemetry.log`; action none | `HQ_OTLP_TOKEN` | `fixtures/otlp/` | — |
| `prometheus` | Rust | bearer/basic | events `telemetry.metric`; action `query` | `PROMETHEUS_URL`, `PROMETHEUS_TOKEN`, `PROMETHEUS_USER`, `PROMETHEUS_PASSWORD` | `fixtures/prometheus/` | `HQ_LIVE_PROMETHEUS=1` |
| `notion` | Rust | integration token | events `knowledge.page_synced`, `knowledge.page_quarantined`; actions `search`, `fetch_page`, `sync` | `NOTION_TOKEN` | `fixtures/notion/` | `HQ_LIVE_NOTION=1` |
| `eodhd` | Rust | API key query | events `quote.observed`, `fundamental.observed`; action `fetch_quote` | `EODHD_API_KEY` | `fixtures/eodhd/` | `HQ_LIVE_EODHD=1` |
| `fmp` | Rust | API key query | events `quote.observed`, `fundamental.observed`; action `fetch_quote` | `FMP_API_KEY` | `fixtures/fmp/` | `HQ_LIVE_FMP=1` |
| `heartbeat` | Rust | agent bearer token per slug | events `agent.seen`, `agent.observed`, `agent.disappeared`; action none | `HQ_HEARTBEAT_TOKEN`, `${secret:agent.<slug>.token}` | `fixtures/heartbeat/` | — |
| `local` | Rust | none (runtime lokal) | events `agent.*`, `session.*`, `usage.recorded`, `quota.observed`, `log.recorded`; action none (control plane) | `HQ_LOCAL_ADAPTERS`, `HQ_MUSE_PEER`, `DEVIN_API_KEY` untuk adapter Devin | `fixtures/local/`, `fixtures/runtime/` | — |

### 5.3 Tier 2 preset (YAML)

| id | auth | event / action | kredensial | fixture | live |
|---|---|---|---|---|---|
| `slack` | bearer bot token | events `message.received`, `app_mention`, `reaction.added`; actions `send_message`, `update_message`, `add_reaction` | `SLACK_BOT_TOKEN`, `SLACK_SIGNING_SECRET` | `fixtures/slack/` | `HQ_LIVE_SLACK=1` |
| `discord` | bot token | events `message.received`, `interaction.received`; action `send_message` | `DISCORD_BOT_TOKEN`, `DISCORD_PUBLIC_KEY` | `fixtures/discord/` | `HQ_LIVE_DISCORD=1` |
| `whatsapp_cloud` | bearer Meta Cloud | events `message.received`, `status.updated`; actions `send_message`, `send_template` | `WHATSAPP_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_VERIFY_TOKEN`, `WHATSAPP_APP_SECRET` | `fixtures/whatsapp_cloud/` | `HQ_LIVE_WHATSAPP=1` |
| `linear` | API key atau OAuth2 | events `issue.created`, `issue.updated`, `comment.created`; actions `create_issue`, `add_comment` | `LINEAR_API_KEY`, `LINEAR_WEBHOOK_SECRET` | `fixtures/linear/` | `HQ_LIVE_LINEAR=1` |
| `jira` | basic (email + API token) | events `issue.created`, `issue.updated`; actions `create_issue`, `transition_issue`, `add_comment` | `JIRA_BASE_URL`, `JIRA_EMAIL`, `JIRA_API_TOKEN`, `JIRA_WEBHOOK_SECRET` | `fixtures/jira/` | `HQ_LIVE_JIRA=1` |
| `sentry` | auth token | events `issue.created`, `issue.resolved`, `event.alert`; actions `resolve_issue`, `assign_issue` | `SENTRY_AUTH_TOKEN`, `SENTRY_ORG`, `SENTRY_PROJECT`, `SENTRY_CLIENT_SECRET` | `fixtures/sentry/` | `HQ_LIVE_SENTRY=1` |
| `home_assistant` | long-lived token | events `state.changed`; action `call_service` | `HASS_URL`, `HASS_TOKEN` | `fixtures/home_assistant/` | `HQ_LIVE_HASS=1` |
| `google_calendar` | OAuth2 authorization-code | events `event.created`, `event.updated`, `event.starting`; action `create_event` | `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `${secret:connector.google_calendar.refresh_token}` | `fixtures/google_calendar/` | `HQ_LIVE_GCAL=1` |
| `smtp` | basic atau none (relay lokal) | events none; action `send_email` | `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM` | `fixtures/smtp/` | `HQ_LIVE_SMTP=1` |
| `ntfy` | bearer atau none | events `message.received`; action `publish` | `NTFY_URL`, `NTFY_TOKEN`, `NTFY_TOPIC` | `fixtures/ntfy/` | `HQ_LIVE_NTFY=1` |
| `webhook` | HMAC secret | event `connector.event`; action `send` | `WEBHOOK_SECRET`, `WEBHOOK_URL` | `fixtures/webhook/` | `HQ_LIVE_WEBHOOK=1` |
| `pagerduty` | API token + routing key | events `incident.triggered`, `incident.acknowledged`, `incident.resolved`; action `trigger_incident` | `PAGERDUTY_TOKEN`, `PAGERDUTY_ROUTING_KEY`, `PAGERDUTY_WEBHOOK_SECRET` | `fixtures/pagerduty/` | `HQ_LIVE_PAGERDUTY=1` |
| `teams` | OAuth2 client-credentials atau webhook URL | events `message.received`; action `send_message` | `TEAMS_TENANT_ID`, `TEAMS_CLIENT_ID`, `TEAMS_CLIENT_SECRET`, `TEAMS_WEBHOOK_URL` | `fixtures/teams/` | `HQ_LIVE_TEAMS=1` |
| `lark` | app id + app secret | events `message.received`; action `send_message` | `LARK_APP_ID`, `LARK_APP_SECRET`, `LARK_VERIFICATION_TOKEN` | `fixtures/lark/` | `HQ_LIVE_LARK=1` |
| `mattermost` | bot token | events `message.received`; action `send_message` | `MATTERMOST_URL`, `MATTERMOST_TOKEN` | `fixtures/mattermost/` | `HQ_LIVE_MATTERMOST=1` |
| `gotify` | app token | events none; action `send` | `GOTIFY_URL`, `GOTIFY_TOKEN` | `fixtures/gotify/` | `HQ_LIVE_GOTIFY=1` |
| `pushover` | app token + user key | events none; action `send` | `PUSHOVER_TOKEN`, `PUSHOVER_USER` | `fixtures/pushover/` | `HQ_LIVE_PUSHOVER=1` |
| `twilio` | account SID + auth token | events `message.status`; action `send_sms` | `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, `TWILIO_FROM`, `TWILIO_WEBHOOK_SECRET` | `fixtures/twilio/` | `HQ_LIVE_TWILIO=1` |
| `resend` | API key | events `email.delivered`, `email.bounced`; action `send_email` | `RESEND_API_KEY`, `RESEND_FROM`, `RESEND_WEBHOOK_SECRET` | `fixtures/resend/` | `HQ_LIVE_RESEND=1` |
| `vercel` | bearer token | events `deployment.created`, `deployment.ready`, `deployment.error`; action `redeploy` | `VERCEL_TOKEN`, `VERCEL_TEAM_ID`, `VERCEL_WEBHOOK_SECRET` | `fixtures/vercel/` | `HQ_LIVE_VERCEL=1` |
| `cloudflare` | API token | events `alert.received`; action `purge_cache` | `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ZONE_ID`, `CLOUDFLARE_WEBHOOK_SECRET` | `fixtures/cloudflare/` | `HQ_LIVE_CLOUDFLARE=1` |
| `rss` | none | events `feed.item`; action none | `RSS_FEEDS` / `${config:rss.feed_url}` | `fixtures/rss/` | `HQ_LIVE_RSS=1` |
| `docker` | unix socket atau TLS | events `container.started`, `container.stopped`, `container.died`, `container.health_changed`; actions `start_container`, `stop_container`, `restart_container` | `DOCKER_HOST`, `DOCKER_TLS_VERIFY`, `DOCKER_CERT_PATH` | `fixtures/docker/` | `HQ_LIVE_DOCKER=1` |

### 5.4 Tier 3 generic

| id | bentuk | auth | event / action | kredensial | fixture | live |
|---|---|---|---|---|---|---|
| `http` | YAML | none/bearer/basic/api-key/OAuth2 | event `connector.event`; action `request` | `${config:http.*}`, `${secret:connector.http.*}` | `fixtures/http/` | `HQ_LIVE_HTTP=1` |
| `mcp` | Rust + YAML config | stdio/HTTP bearer | events `mcp.tools_observed`, `mcp.tool_called`; actions `list_tools`, `call_tool` | config per server; `MCP_HTTP_TOKEN` | `fixtures/mcp/` | `HQ_LIVE_MCP=1` |

`mcp` sengaja muncul di Tier 1 dan Tier 3: implementasinya satu native client,
tetapi server apa pun dapat dikonfigurasi tanpa kode baru. Tidak ada id kedua
dan tidak ada implementasi kedua.

## 6. Definisi selesai connector

Connector dianggap selesai hanya bila semua baris ini benar:

| Syarat | Bukti |
|---|---|
| Manifest ada dan lolos `validate_manifest` | `cargo test -p hq-connectors manifest_corpus` |
| Tier dan bentuk jujur | `native` punya Rust; `preset`/`generic` YAML + engine |
| Auth terdokumentasi | `docs/connectors/<id>.md`: mode, scope, env, rotasi |
| Tes sukses | `<id>_success` |
| Tes 401 | `<id>_unauthorized` |
| Tes payload rusak | `<id>_malformed` |
| Tes webhook (bila ada) | signature valid/salah, timestamp, replay nonce, duplikat |
| Tes duplikat webhook | `duplicate_webhook_is_noop` (02 §4/08 §2) |
| Health transition | `unconfigured → unknown → healthy`, gagal → `degraded/down` |
| Rate limit + backoff | tes time dipause; tidak ada `sleep` > 50 ms |
| Redaksi | tidak ada secret di event/log/error/DB |
| Fixture | `fixtures/<id>/` lengkap |
| Live smoke | `HQ_LIVE_<ID>=1`, `#[ignore]`, tidak menggagalkan CI tanpa kredensial |
| Dokumen | `docs/connectors/<id>.md` |
| Tidak ada data palsu | tanpa kredensial → `unconfigured` |

### 6.1 Perilaku `unconfigured`

- Health `unconfigured`; `last_error = null`; `next_run_ms = null`.
- Supervisor tidak memanggil `poll` atau `action`; hanya health sweep untuk
  mendeteksi secret baru.
- `POST /api/v1/connectors/:id/test` mengembalikan HTTP 200 dengan
  `{ok:false, code:"unconfigured", missing:["connector.slack.bot_token"]}`.
  `missing` hanya nama path, bukan nilai.
- `POST /api/v1/connectors/:id/actions/:action` mengembalikan 409
  `unconfigured`; tidak ada request keluar.
- `POST /api/v1/connectors/:id/enable` boleh berhasil, tetapi health tetap
  `unconfigured` sampai secret lengkap.
- API `GET /connectors` menampilkan `unconfigured`, tier, dan daftar path
  secret yang hilang; tidak ada event data palsu.
- `connector.health_changed` dipancarkan sekali saat masuk `unconfigured`;
  tidak berulang setiap tick.
- `hq doctor` menampilkan connector `unconfigured` beserta perbaikan, bukan
  error fatal.

## 7. Interface dan tes S17–S23

### 7.1 S17 — framework, engine, secret

**Produces**

```rust
pub fn load_manifest(path: &std::path::Path) -> Result<Manifest, ManifestError>;
pub fn validate_manifest(m: &Manifest) -> Result<(), Vec<ValidationError>>;

pub struct Engine;

impl Engine {
    pub async fn poll(
        &self,
        m: &Manifest,
        ctx: &ConnectorCtx<'_>,
    ) -> Result<Vec<EventDraft>, ConnectorError>;

    pub async fn handle_webhook(
        &self,
        m: &Manifest,
        req: WebhookRequest,
        ctx: &ConnectorCtx<'_>,
    ) -> Result<Vec<EventDraft>, ConnectorError>;

    pub async fn action(
        &self,
        m: &Manifest,
        name: &str,
        input: serde_json::Value,
        ctx: &ConnectorCtx<'_>,
    ) -> Result<ActionOutcome, ConnectorError>;
}

pub struct Supervisor;

impl Supervisor {
    pub fn new(
        runtimes: Vec<Arc<ConnectorRuntime>>,
        sink: Arc<dyn ConnectorSink>,
    ) -> Self;

    pub async fn run(self, shutdown: tokio::sync::watch::Receiver<bool>);

    pub async fn run_once(&self, id: &str) -> Result<RunReport, ConnectorError>;
}
```

**Tes wajib**

| Tes | Acceptance |
|---|---|
| `manifest_corpus_valid_invalid` | corpus `tests/corpus/manifests/*.yaml`; kode error cocok |
| `manifest_rejects_plaintext_secret` | token di YAML → `secret.plaintext_forbidden` |
| `secret_roundtrip_age` | set → list fingerprint → get; nilai sama |
| `secret_file_permissions_enforced` | `0644` → error, bukan warning |
| `secret_rotate_emits_no_value` | event hanya key_id/path hash |
| `supervisor_backoff_jitter` | failure 1..6 → delay sesuai formula; jitter ±20% |
| `supervisor_no_overlapping_run` | poll lambat + tick berikutnya → satu run, satu skip |
| `supervisor_circuit_breaker_opens` | 5 gagal → `down`; half-open 60 s |
| `supervisor_rate_limit_defers` | bucket kosong → `next_run_ms` mundur |
| `engine_maps_jsonpath` | mapping fixture → event_type/data tepat |
| `engine_pagination_cursor` | 2 halaman → 2 batch, cursor disimpan |
| `engine_auth_bearer` | header `Authorization` benar; secret tidak di log |
| `engine_oauth_refresh` | token kedaluwarsa → refresh sekali; 401 kedua → Auth |
| `webhook_hmac_valid` | signature valid → 204 |
| `webhook_hmac_invalid` | signature salah → 401 |
| `webhook_timestamp_expired` | skew > batas → 401 |
| `webhook_nonce_replay` | nonce sama → 409 |
| `duplicate_webhook_is_noop` | payload sama → satu event, 204 |
| `unconfigured_connector_skips_poll` | tanpa secret → tidak ada request, health `unconfigured` |
| `late_event_updates_bucket` | event telat → bucket waktu diperbarui, bukan event baru palsu |

### 7.2 S18–S23

| Sesi | Interface / deliverable | Tes wajib |
|---|---|---|
| S18 GitHub | `GitHubConnector` implements `Connector`; webhook signature + delivery nonce | `github_success`, `github_unauthorized`, `github_malformed`, `github_webhook_signature_invalid`, `github_webhook_replay`, `github_webhook_duplicate_is_noop`, `github_live` |
| S19 Telegram | `TelegramConnector`; action send/edit; webhook secret token; approval callback | `telegram_success`, `telegram_unauthorized`, `telegram_malformed`, `telegram_webhook_secret_invalid`, `telegram_duplicate_webhook_is_noop`, `telegram_approval_roundtrip`, `telegram_live` |
| S20 MCP | `McpClient::{list_tools, call_tool}`; `McpServer` read-only default | `mcp_tools_list`, `mcp_tool_call_policy_denied`, `mcp_server_roundtrip`, `mcp_unauthorized`, `mcp_malformed`, `mcp_live` |
| S21 OTLP/Prometheus | `OtlpReceiver::{ingest_metrics, ingest_traces, ingest_logs}`; `PrometheusConnector` | `otlp_metrics_ingest`, `otlp_traces_ingest`, `otlp_logs_ingest`, `otlp_bad_payload`, `prometheus_scrape`, `prometheus_unauthorized`, `prometheus_live` |
| S22 preset batch 1 | manifest + engine; prefix `slack`, `discord`, `whatsapp_cloud`, `smtp`, `ntfy`, `webhook` | `<prefix>_success`, `<prefix>_unauthorized`, `<prefix>_malformed`; webhook prefix juga `<prefix>_webhook_signature_invalid`, `<prefix>_webhook_replay`, `<prefix>_duplicate_webhook_is_noop`; `<prefix>_live` |
| S23 preset batch 2 | manifest + engine; prefix `linear`, `jira`, `sentry`, `home_assistant`, `google_calendar`, `pagerduty`, `teams`, `lark`, `mattermost`, `gotify`, `pushover`, `twilio`, `resend`, `vercel`, `cloudflare`, `rss`, `docker` | pola yang sama; `gotify`, `pushover`, `rss`, `docker` tanpa webhook hanya tiga tes dasar; `docker` memakai `<prefix>_success` untuk daftar container dan `<prefix>_action_start` untuk start |

Di S22, `smtp` dan `ntfy` tidak punya webhook; keduanya hanya tiga tes dasar.
`webhook` sendiri selalu punya webhook.

Aturan penamaan lengkap untuk S22/S23: `<prefix>` adalah id connector dengan
`_` sebagai pengganti `-`. Setiap connector wajib punya minimal
`<prefix>_success`, `<prefix>_unauthorized`, `<prefix>_malformed`. Yang punya
webhook wajib menambah `<prefix>_webhook_signature_invalid`,
`<prefix>_webhook_replay`, dan `<prefix>_duplicate_webhook_is_noop`. Yang punya
aksi wajib menambah `<prefix>_action_<name>`. Live smoke selalu
`<prefix>_live` dan `#[ignore]`.

`duplicate_webhook_is_noop` dan `late_event_updates_bucket` adalah nama yang
sama dengan 08 §2; connector boleh memakai prefiks
(`<prefix>_duplicate_webhook_is_noop`) tetapi tes global dengan nama tanpa
prefiks tetap wajib ada di S17.

## Diserahkan ke builder

- OAuth2 authorization-code membutuhkan halaman redirect yang menampilkan
  `code`. Dokumen ini menetapkan action `oauth_authorize`/`oauth_exchange` dan
  penyimpanan token, tetapi memilih UX redirect (loopback sementara vs halaman
  Integrations) diserahkan ke builder; jangan menambah endpoint API di luar
  01 §5.
- Docker API memakai unix socket; engine HTTP `reqwest` butuh dukungan unix
  socket. Jika builder tidak menambah dukungan itu di engine, `docker` menjadi
  connector native, bukan YAML. Tier katalog berubah bersama keputusan itu.
- `mcp` adalah satu implementasi yang melayani Tier 1 dan Tier 3. Builder
  memilih apakah manifest `mcp` menampilkan daftar server sebagai child
  `agent:mcp/<slug>` di Integrations atau hanya di detail agent.
- 01 §1 belum mencantumkan `serde`, `thiserror`, `async-trait`, `serde_yaml`,
  dan `jsonpath-rust` di daftar dependency `hq-connectors`. Builder menambahkannya
  saat S17 atau mengekspos re-export dari `hq-core`; jangan memakai transitive
  dependency tanpa deklarasi.
- Nonce store memakai tabel `settings` dengan prefiks `nonce:<connector>:<nonce>`
  dan sweeper TTL. Jika builder memilih tabel khusus, 01 §3 harus diperbarui
  lebih dulu.
- Engine v1 hanya memverifikasi HMAC (sesuai 00 §11.2). Discord interaction
  memakai Ed25519; builder memilih menambah `ed25519-dalek` + varian algoritma
  (dan memperbarui 01 §1) atau membuat verifikasi Discord native tanpa engine.
