//! Pengetahuan HQ dari Notion: cermin baca-saja yang disinkronkan berkala.
//!
//! Desain (SA `~/workspace/work/hq-notion-knowledge-sa-2026-11-07.md`):
//! - Sync periodik inkremental (scheduler 30 menit + trigger manual), BUKAN
//!   on-demand fetch, BUKAN webhook. Backfill pertama ber-checkpoint.
//! - Dua lapis: `notion_docs` (cermin mentah, status `synced`) TERPISAH dari
//!   `memory_entries` (B.5 — hanya manusia yang menulis `trusted`).
//! - FTS5 (BM25) untuk keduanya; tanpa embedding/vector DB/layanan berbayar.
//! - Klien Notion = reqwest langsung ke api.notion.com/v1 (pola stocks).
//! - KEAMANAN (keras): teks Notion adalah DATA, bukan instruksi — TIDAK ADA
//!   teks mentah Notion di paket dispatch v1. Halaman yang mengandung pola
//!   rahasia DIKARANTINA (tidak disimpan, bahkan tidak dalam keadaan
//!   tersensor). Sync TIDAK PERNAH menulis `memory_entries` (bukti grep:
//!   tidak ada `INSERT INTO memory_entries` di file ini di luar jalur
//!   promosi manusia — dan jalur promosi pun hanya menulis `candidate`).
//! - Token HANYA dari env `NOTION_API_KEY`; tanpa token modul jalan dalam
//!   mode mock (fixture lokal) dengan banner jujur di panel.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use hq_core::redact::{contains_secret, redact};
use hq_core::timefmt::now_ms;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;

/// Pin versi API Notion (stabil, paling luas didukung — SA §1.3).
const NOTION_API_VERSION_DEFAULT: &str = "2022-06-28";
/// Interval sync default (menit) — SA D1.
const SYNC_INTERVAL_MIN_DEFAULT: u64 = 30;
/// Rate limiter konservatif: 2 req/detik (di bawah batas 3/detik Notion,
/// menyisakan headroom untuk MCP Melo) — SA §1.3/§6.2.
const MIN_REQUEST_GAP_MS: u64 = 500;
/// Percobaan maksimum per panggilan HTTP (429/529/network).
const MAX_ATTEMPTS: u32 = 3;
/// Batas halaman per sumber saat backfill (pengaman loop).
const MAX_PAGES_PER_SOURCE: usize = 1000;
/// Kedalaman maksimum walk subtree saat backfill.
const MAX_WALK_DEPTH: usize = 10;

// ------------------------------------------------------------------- util

fn env_key(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v.trim() == "1" || v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// SHA-256 hex dari teks (detektor perubahan konten — SA §4.1).
fn sha256_hex(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

/// Parse RFC3339 (`2026-11-07T08:00:00.000Z` / `+07:00`) -> epoch ms.
/// Mengembalikan None bila format tidak dikenali (pemanggil memakai 0).
fn parse_rfc3339_ms(s: &str) -> Option<i64> {
    let (date_part, rest) = s.split_once('T')?;
    let mut di = date_part.split('-');
    let y: i64 = di.next()?.parse().ok()?;
    let mo: i64 = di.next()?.parse().ok()?;
    let d: i64 = di.next()?.parse().ok()?;
    // Pisahkan waktu dari zona (Z atau ±HH:MM).
    let (time_part, offset_min) = if let Some(t) = rest.strip_suffix('Z') {
        (t, 0i64)
    } else if let Some(idx) = rest.rfind(['+', '-']) {
        let (t, z) = rest.split_at(idx);
        let sign = if z.starts_with('-') { -1i64 } else { 1i64 };
        let mut zi = z[1..].split(':');
        let zh: i64 = zi.next()?.parse().ok()?;
        let zm: i64 = zi.next().unwrap_or("0").parse().ok()?;
        (t, sign * (zh * 60 + zm))
    } else {
        (rest, 0i64)
    };
    // Pecah waktu + pecahan detik (.123 -> 123 ms; Notion mengirim pecahan).
    let (time_main, frac_ms) = match time_part.split_once('.') {
        Some((m, f)) => {
            let digits: String = f.chars().take_while(|c| c.is_ascii_digit()).collect();
            let mut ms: i64 = digits.parse().unwrap_or(0);
            match digits.len() {
                1 => ms *= 100,
                2 => ms *= 10,
                4.. => ms /= 10i64.pow((digits.len() - 3) as u32),
                _ => {}
            }
            (m, ms)
        }
        None => (time_part, 0),
    };
    let mut ti = time_main.split(':');
    let h: i64 = ti.next()?.parse().ok()?;
    let mi: i64 = ti.next()?.parse().ok()?;
    let sec: i64 = ti.next().unwrap_or("0").parse().ok()?;
    // days-from-civil (Howard Hinnant, kebalikan dari wib_parts).
    let y_adj = if mo <= 2 { y - 1 } else { y };
    let era = y_adj.div_euclid(400);
    let yoe = y_adj.rem_euclid(400);
    let mp = if mo > 2 { mo - 3 } else { mo + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let secs = days * 86400 + h * 3600 + mi * 60 + sec - offset_min * 60;
    Some(secs * 1000 + frac_ms)
}

/// Escape query pengguna untuk FTS5 MATCH: tiap token dikutip, digabung AND.
/// Tanpa ini, karakter seperti `"` atau `*` dari input bisa merusak MATCH.
fn fts_escape(q: &str) -> String {
    let tokens: Vec<String> = q
        .split_whitespace()
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();
    if tokens.is_empty() {
        "\"\"".to_string()
    } else {
        tokens.join(" AND ")
    }
}

// ------------------------------------------------------------------- config

pub struct NotionConfig {
    pub data_dir: String,
    notion_key: Option<String>,
    notion_base: String,
    notion_version: String,
    sync_interval_min: u64,
    mock_forced: bool,
}

/// Debug tanpa kunci API (kunci tidak boleh ikut tercetak).
impl std::fmt::Debug for NotionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotionConfig")
            .field("data_dir", &self.data_dir)
            .field(
                "notion_key",
                &self.notion_key.as_ref().map(|_| "<dari env>"),
            )
            .field("notion_base", &self.notion_base)
            .field("notion_version", &self.notion_version)
            .field("sync_interval_min", &self.sync_interval_min)
            .field("mock_forced", &self.mock_forced)
            .finish()
    }
}

impl NotionConfig {
    pub fn from_env(data_dir: &str) -> Self {
        let base = |name: &str, default: &str| {
            std::env::var(name)
                .ok()
                .map(|v| v.trim().trim_end_matches('/').to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| default.to_string())
        };
        let interval = std::env::var("NOTION_SYNC_INTERVAL_MIN")
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .filter(|v| *v >= 5)
            .unwrap_or(SYNC_INTERVAL_MIN_DEFAULT);
        NotionConfig {
            data_dir: data_dir.to_string(),
            notion_key: env_key("NOTION_API_KEY"),
            notion_base: base("NOTION_API_BASE", "https://api.notion.com/v1"),
            notion_version: base("NOTION_API_VERSION", NOTION_API_VERSION_DEFAULT),
            sync_interval_min: interval,
            mock_forced: env_flag("HQ_NOTION_MOCK"),
        }
    }

    /// "live" bila ada token dan mock tidak dipaksa; "mock" bila tidak.
    pub fn mode(&self) -> &'static str {
        if self.mock_forced || self.notion_key.is_none() {
            "mock"
        } else {
            "live"
        }
    }

    pub fn configured(&self) -> bool {
        self.notion_key.is_some()
    }
}

// ------------------------------------------------------------------- error

#[derive(Debug, Clone)]
pub enum NotionError {
    /// 401 — token salah/dicabut.
    Auth,
    /// 404 — halaman tidak dibagikan ke integrasi / tidak ada.
    NotFound,
    /// 429/529 + Retry-After (detik).
    RateLimited(Option<u64>),
    /// 402/403 — kuota habis / fitur berbayar.
    Quota,
    Network(&'static str),
    Server(u16),
    BadBody,
}

impl NotionError {
    /// Pesan aman untuk log/JSON — tidak pernah memuat URL atau kunci.
    fn safe_msg(&self) -> String {
        match self {
            NotionError::Auth => "token Notion tidak valid (401) — periksa NOTION_API_KEY".into(),
            NotionError::NotFound => {
                "halaman tidak ditemukan / belum dibagikan ke integrasi rizal-agent-hq (404)".into()
            }
            NotionError::RateLimited(_) => {
                "Notion membatasi permintaan (429) — dicoba ulang".into()
            }
            NotionError::Quota => "kuota/fitur Notion ditolak (402/403)".into(),
            NotionError::Network(m) => format!("jaringan: {m}"),
            NotionError::Server(s) => format!("Notion membalas {s}"),
            NotionError::BadBody => "balasan Notion tidak dikenal".into(),
        }
    }
}

/// Pesan error reqwest tanpa URL (URL memuat token di header, bukan query —
/// tetapi tetap jangan pernah mencetaknya).
fn safe_net_err(e: &reqwest::Error) -> NotionError {
    if e.is_timeout() {
        NotionError::Network("timeout")
    } else if e.is_connect() {
        NotionError::Network("koneksi gagal")
    } else {
        NotionError::Network("permintaan gagal")
    }
}

fn status_err(status: u16, retry_after: Option<u64>) -> Option<NotionError> {
    match status {
        200..=299 => None,
        401 => Some(NotionError::Auth),
        402 | 403 => Some(NotionError::Quota),
        404 => Some(NotionError::NotFound),
        429 | 529 => Some(NotionError::RateLimited(retry_after)),
        s => Some(NotionError::Server(s)),
    }
}

// ------------------------------------------------------------------- model

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMeta {
    pub page_id: String,
    pub title: String,
    pub parent_page_id: Option<String>,
    pub last_edited_ms: i64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDoc {
    pub page_id: String,
    pub title: String,
    #[serde(default)]
    pub parent_path: String,
    #[serde(default)]
    pub group_slug: String,
    pub content_text: String,
    pub notion_last_edited_ms: i64,
    pub notion_url: String,
    #[serde(default)]
    pub child_page_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub root_page_id: String,
    pub title: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourcesFile {
    #[serde(default)]
    sources: Vec<Source>,
}

/// Daftar sumber (allowlist) — dari `data/notion.json`, BUKAN hardcode.
/// Fallback: env `NOTION_ROOT_PAGES` (`id:page_id` dipisah koma).
/// Bila keduanya tidak ada: sumber mock (id saja, tanpa page ID nyata —
/// klien mock mengabaikan root dan melayani fixture berdasarkan group).
pub fn load_sources(data_dir: &str) -> Vec<Source> {
    let path = format!("{data_dir}/notion.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(parsed) = serde_json::from_str::<SourcesFile>(&text) {
            let list: Vec<Source> = parsed
                .sources
                .into_iter()
                .filter(|s| !s.id.trim().is_empty() && !s.root_page_id.trim().is_empty())
                .collect();
            if !list.is_empty() {
                return list;
            }
        }
    }
    if let Some(env) = env_key("NOTION_ROOT_PAGES") {
        let list: Vec<Source> = env
            .split(',')
            .filter_map(|part| {
                let (id, page) = part.split_once(':')?;
                let id = id.trim().to_string();
                let page = page.trim().to_string();
                if id.is_empty() || page.is_empty() {
                    None
                } else {
                    Some(Source {
                        id: id.clone(),
                        root_page_id: page,
                        title: id,
                        enabled: true,
                    })
                }
            })
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    vec![
        Source {
            id: "hq".into(),
            root_page_id: String::new(),
            title: "Rizal Agent HQ".into(),
            enabled: true,
        },
        Source {
            id: "strutstack".into(),
            root_page_id: String::new(),
            title: "StrutStack".into(),
            enabled: true,
        },
        Source {
            id: "tracerio".into(),
            root_page_id: String::new(),
            title: "Tracerio".into(),
            enabled: true,
        },
    ]
}

// ------------------------------------------------------------------- skema

/// 4 tabel Notion (§4.1 SA) + 3 tabel memori B.5 §7.2 (verbatim) + FTS5.
/// `CREATE TABLE IF NOT EXISTS` — `hq.db` lama tetap bisa dibuka.
pub fn ensure_notion_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS notion_docs(
            page_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            parent_path TEXT NOT NULL,
            group_slug TEXT NOT NULL,
            content_text TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            word_count INTEGER NOT NULL,
            notion_url TEXT NOT NULL,
            notion_last_edited INTEGER NOT NULL,
            synced_at INTEGER NOT NULL,
            sync_status TEXT NOT NULL,
            quarantine_note TEXT
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS notion_docs_fts
            USING fts5(title, content_text, content='notion_docs', content_rowid='rowid',
                       tokenize='unicode61');
        CREATE TABLE IF NOT EXISTS notion_sync_state(
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS notion_sources(
            id TEXT PRIMARY KEY,
            root_page_id TEXT NOT NULL,
            title TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            include_subtree INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS memory_entries(
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            scope TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            status TEXT NOT NULL,
            evidence TEXT,
            supersedes_id TEXT,
            valid_from INTEGER NOT NULL,
            valid_to INTEGER,
            created_by TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            promoted_by TEXT,
            promoted_at INTEGER,
            use_count INTEGER NOT NULL DEFAULT 0,
            last_used_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS memory_events(
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            at INTEGER NOT NULL,
            entry_id TEXT NOT NULL,
            action TEXT NOT NULL,
            actor TEXT NOT NULL,
            note TEXT
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS memory_entries_fts
            USING fts5(title, body, content='memory_entries', content_rowid='rowid',
                       tokenize='unicode61');",
    )
    .expect("skema notion/memory");
}

fn get_state(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM notion_sync_state WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

fn set_state(conn: &Connection, key: &str, value: &str) {
    let _ = conn.execute(
        "INSERT INTO notion_sync_state(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    );
}

/// Sinkronkan ulang baris FTS5 external-content untuk satu halaman.
/// (FTS5 external-content tidak ter-update otomatis saat tabel isi berubah.)
/// Bangun ulang indeks FTS5 dari tabel konten.
///
/// PENTING: tabel FTS di sini memakai `content='<tabel>'` (external
/// content). Perawatan per-baris (DELETE + INSERT langsung ke tabel FTS)
/// MERUSAK indeks — `snippet()`/`bm25()` kemudian gagal dengan
/// SQLITE_CORRUPT ("database disk image is malformed") meski MATCH
/// sederhana masih jalan dan `integrity_check` bersih. Satu-satunya
/// perawatan yang benar untuk tabel external-content adalah perintah
/// khusus `rebuild` (terverifikasi langsung pada DB hasil sync).
/// Dipanggil sekali per sync / sekali per mutasi memori, bukan per halaman.
fn rebuild_fts(conn: &Connection, table_fts: &str) {
    let _ = conn.execute(
        &format!("INSERT INTO {table_fts}({table_fts}) VALUES('rebuild')"),
        [],
    );
}

// ------------------------------------------------------------------- simpan

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreOutcome {
    Updated,
    Unchanged,
    Quarantined,
}

/// Simpan satu dokumen hasil sync (satu transaksi per halaman).
/// Gerbang karantina dinilai pada teks ASLI: bila mengandung pola rahasia,
/// isi TIDAK disimpan (bahkan tidak tersensor) — B.5 §7.1. Teks yang lolos
/// tetap melewati `redact()` sebagai pass kedua sebelum disimpan.
fn store_page(conn: &Connection, doc: &PageDoc, now: i64) -> StoreOutcome {
    if contains_secret(&doc.content_text) {
        let _ = conn.execute(
            "INSERT INTO notion_docs(page_id, title, parent_path, group_slug, content_text, content_hash, word_count, notion_url, notion_last_edited, synced_at, sync_status, quarantine_note)
             VALUES(?1,?2,?3,?4,'','',0,?5,?6,?7,'quarantined','mengandung pola rahasia')
             ON CONFLICT(page_id) DO UPDATE SET title=excluded.title, parent_path=excluded.parent_path, group_slug=excluded.group_slug,
                 content_text='', content_hash='', word_count=0, notion_url=excluded.notion_url,
                 notion_last_edited=excluded.notion_last_edited, synced_at=excluded.synced_at,
                 sync_status='quarantined', quarantine_note='mengandung pola rahasia'",
            rusqlite::params![
                doc.page_id,
                doc.title,
                doc.parent_path,
                doc.group_slug,
                doc.notion_url,
                doc.notion_last_edited_ms,
                now
            ],
        );
        // Indeks FTS dibangun ulang sekali di akhir run_sync (lihat rebuild_fts).
        set_state(
            conn,
            &format!("quarantined:{}", doc.page_id),
            "pola rahasia terdeteksi",
        );
        return StoreOutcome::Quarantined;
    }
    let text = redact(&doc.content_text);
    let hash = sha256_hex(&text);
    let words = text.split_whitespace().count() as i64;
    let existing: Option<String> = conn
        .query_row(
            "SELECT content_hash FROM notion_docs WHERE page_id = ?1",
            rusqlite::params![doc.page_id],
            |r| r.get::<_, String>(0),
        )
        .ok();
    if existing.as_deref() == Some(hash.as_str()) {
        let _ = conn.execute(
            "UPDATE notion_docs SET synced_at = ?1, notion_last_edited = ?2, sync_status = 'synced', quarantine_note = NULL WHERE page_id = ?3",
            rusqlite::params![now, doc.notion_last_edited_ms, doc.page_id],
        );
        return StoreOutcome::Unchanged;
    }
    let _ = conn.execute(
        "INSERT INTO notion_docs(page_id, title, parent_path, group_slug, content_text, content_hash, word_count, notion_url, notion_last_edited, synced_at, sync_status, quarantine_note)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'synced',NULL)
         ON CONFLICT(page_id) DO UPDATE SET title=excluded.title, parent_path=excluded.parent_path, group_slug=excluded.group_slug,
             content_text=excluded.content_text, content_hash=excluded.content_hash, word_count=excluded.word_count,
             notion_url=excluded.notion_url, notion_last_edited=excluded.notion_last_edited, synced_at=excluded.synced_at,
             sync_status='synced', quarantine_note=NULL",
        rusqlite::params![
            doc.page_id,
            doc.title,
            doc.parent_path,
            doc.group_slug,
            text,
            hash,
            words,
            doc.notion_url,
            doc.notion_last_edited_ms,
            now
        ],
    );
    // Indeks FTS dibangun ulang sekali di akhir run_sync (lihat rebuild_fts).
    StoreOutcome::Updated
}

// ------------------------------------------------------------------- klien

/// Statistik HTTP (untuk `GET /api/knowledge/sync/status`): jumlah request
/// dalam 60 detik terakhir.
#[derive(Default)]
struct HttpStats {
    hits: std::sync::Mutex<VecDeque<Instant>>,
}

impl HttpStats {
    fn record(&self) {
        if let Ok(mut h) = self.hits.lock() {
            let now = Instant::now();
            h.push_back(now);
            while h
                .front()
                .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(60))
            {
                h.pop_front();
            }
        }
    }

    fn rpm(&self) -> usize {
        if let Ok(mut h) = self.hits.lock() {
            let now = Instant::now();
            while h
                .front()
                .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(60))
            {
                h.pop_front();
            }
            h.len()
        } else {
            0
        }
    }
}

pub trait NotionClient: Send + Sync {
    async fn search_changed(&self, since_ms: Option<i64>) -> Result<Vec<PageMeta>, NotionError>;
    async fn fetch_page(&self, page_id: &str) -> Result<PageDoc, NotionError>;
    /// Daftar dokumen di bawah root (backfill). Default: semua halaman yang
    /// berubah + fetch isinya (dipakai mock — fixture adalah keseluruhan
    /// korpus). Live meng-override dengan walk subtree yang mengambil tiap
    /// dokumen SATU kali (tanpa fetch ulang di engine).
    async fn list_subtree(&self, _root_page_id: &str) -> Result<Vec<PageDoc>, NotionError> {
        let metas = self.search_changed(None).await?;
        let mut docs = vec![];
        for m in metas {
            docs.push(self.fetch_page(&m.page_id).await?);
        }
        Ok(docs)
    }
}

/// Klien mock: melayani fixture lokal yang DI-EMBED ke binary
/// (`include_str!`) — mode mock tidak butuh file fixture di runtime.
pub struct MockNotionClient {
    pages: Vec<PageDoc>,
}

impl MockNotionClient {
    pub fn from_fixtures() -> Self {
        let raw = [
            include_str!("../tests/fixtures/notion/hq-status.json"),
            include_str!("../tests/fixtures/notion/strutstack-prd.json"),
            include_str!("../tests/fixtures/notion/trap-secret.json"),
        ];
        let mut pages = vec![];
        for text in raw {
            if let Ok(doc) = serde_json::from_str::<PageDoc>(text) {
                pages.push(doc);
            }
        }
        MockNotionClient { pages }
    }
}

impl NotionClient for MockNotionClient {
    async fn search_changed(&self, since_ms: Option<i64>) -> Result<Vec<PageMeta>, NotionError> {
        let mut out: Vec<PageMeta> = self
            .pages
            .iter()
            .filter(|p| since_ms.is_none_or(|s| p.notion_last_edited_ms > s))
            .map(|p| PageMeta {
                page_id: p.page_id.clone(),
                title: p.title.clone(),
                parent_page_id: None,
                last_edited_ms: p.notion_last_edited_ms,
                url: p.notion_url.clone(),
            })
            .collect();
        out.sort_by_key(|b| std::cmp::Reverse(b.last_edited_ms));
        Ok(out)
    }

    async fn fetch_page(&self, page_id: &str) -> Result<PageDoc, NotionError> {
        self.pages
            .iter()
            .find(|p| p.page_id == page_id)
            .cloned()
            .ok_or(NotionError::NotFound)
    }
}

/// Klien live: reqwest langsung ke api.notion.com/v1.
pub struct LiveNotionClient {
    http: reqwest::Client,
    base: String,
    version: String,
    key: String,
    /// Instan request terakhir (token bucket 2 req/detik).
    last_request: AsyncMutex<Instant>,
    stats: Arc<HttpStats>,
}

impl LiveNotionClient {
    fn new(
        http: reqwest::Client,
        base: String,
        version: String,
        key: String,
        stats: Arc<HttpStats>,
    ) -> Self {
        LiveNotionClient {
            http,
            base,
            version,
            key,
            last_request: AsyncMutex::new(Instant::now() - Duration::from_secs(1)),
            stats,
        }
    }

    /// Jeda rate limit (2 req/detik sustained) sebelum tiap panggilan.
    async fn throttle(&self) {
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        let gap = Duration::from_millis(MIN_REQUEST_GAP_MS);
        if elapsed < gap {
            tokio::time::sleep(gap - elapsed).await;
        }
        *last = Instant::now();
    }

    /// Satu panggilan HTTP dengan retry 429/529 (hormati Retry-After) +
    /// retry error jaringan — maks 3 percobaan. `build` dipanggil ulang
    /// tiap percobaan agar request dibangun fresh.
    async fn call_with_retry(
        &self,
        build: &(dyn Fn() -> reqwest::RequestBuilder + Send + Sync),
    ) -> Result<reqwest::Response, NotionError> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            self.throttle().await;
            self.stats.record();
            let resp = match build().send().await {
                Ok(r) => r,
                Err(e) => {
                    if attempt < MAX_ATTEMPTS {
                        tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                        continue;
                    }
                    return Err(safe_net_err(&e));
                }
            };
            let status = resp.status().as_u16();
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok());
            match status_err(status, retry_after) {
                None => return Ok(resp),
                Some(NotionError::RateLimited(wait)) if attempt < MAX_ATTEMPTS => {
                    let secs = wait.unwrap_or(2).clamp(1, 60);
                    tokio::time::sleep(Duration::from_secs(secs)).await;
                }
                Some(e) => return Err(e),
            }
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("Authorization", format!("Bearer {}", self.key))
            .header("Notion-Version", self.version.clone())
    }

    async fn get_json(&self, path: &str) -> Result<Value, NotionError> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .call_with_retry(&|| self.auth(self.http.get(url.clone())))
            .await?;
        resp.json::<Value>().await.map_err(|_| NotionError::BadBody)
    }

    async fn post_json(&self, path: &str, body: Value) -> Result<Value, NotionError> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .call_with_retry(&|| self.auth(self.http.post(url.clone()).json(&body)))
            .await?;
        resp.json::<Value>().await.map_err(|_| NotionError::BadBody)
    }

    /// Judul halaman dari `properties`: properti bertipe `title`.
    fn page_title(page: &Value) -> String {
        let props = page.get("properties").and_then(Value::as_object);
        let mut title = String::new();
        if let Some(props) = props {
            for (_, prop) in props {
                if prop.get("type").and_then(Value::as_str) == Some("title") {
                    title = rich_text_of(prop.get("title"));
                    if !title.is_empty() {
                        break;
                    }
                }
            }
        }
        if title.is_empty() {
            title = page
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("Tanpa judul")
                .to_string();
        }
        title
    }

    /// Ambil anak blok satu halaman (satu level paginasi).
    async fn fetch_blocks(&self, page_id: &str) -> Result<Vec<Value>, NotionError> {
        let mut out = vec![];
        let mut cursor: Option<String> = None;
        loop {
            let mut path = format!("/blocks/{page_id}/children?page_size=100");
            if let Some(c) = &cursor {
                path.push_str(&format!("&start_cursor={c}"));
            }
            let body = self.get_json(&path).await?;
            if let Some(results) = body.get("results").and_then(Value::as_array) {
                out.extend(results.iter().cloned());
            }
            let has_more = body
                .get("has_more")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            cursor = body
                .get("next_cursor")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            if !has_more || cursor.is_none() {
                break;
            }
        }
        Ok(out)
    }

    /// Ratakan blok menjadi teks polos. Blok anak di-inline SATU level
    /// (SA §3.1); `child_page` dicatat sebagai turunan, bukan di-inline.
    /// Tanpa rekursi (satu level sudah cukup — anak dari anak tidak
    /// di-inline, hanya `child_page`-nya yang dicatat).
    async fn flatten_blocks(&self, blocks: &[Value]) -> Result<(String, Vec<String>), NotionError> {
        let mut lines: Vec<String> = vec![];
        let mut children: Vec<String> = vec![];
        for block in blocks {
            let btype = block.get("type").and_then(Value::as_str).unwrap_or("");
            match btype {
                "child_page" => {
                    // ID halaman anak = ID blok itu sendiri. Objek
                    // `child_page` API Notion HANYA berisi `title` (tidak
                    // ada field `id`) — kode lama membaca
                    // `block.child_page.id` yang selalu None, sehingga walk
                    // tidak pernah mengikuti satu pun child page (akar
                    // kegagalan A1 2026-11-07; terbukti lewat probe live:
                    // fetch_blocks mengembalikan 12 child_page, fetch_page
                    // mengembalikan 0 children).
                    if let Some(id) = block.get("id").and_then(Value::as_str) {
                        children.push(id.to_string());
                    }
                }
                "child_database" => { /* baris database dikecualikan (SA §2.3) */ }
                "table" => {
                    // Baris tabel = anak blok `table_row`.
                    if let Some(id) = block.get("id").and_then(Value::as_str) {
                        let rows = self.fetch_blocks(id).await?;
                        for row in &rows {
                            if row.get("type").and_then(Value::as_str) == Some("table_row") {
                                let cells: Vec<String> = row
                                    .get("table_row")
                                    .and_then(|tr| tr.get("cells"))
                                    .and_then(Value::as_array)
                                    .map(|cs| {
                                        cs.iter().map(|c| rich_text_of(Some(c))).collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                // @-mention di dalam sel ikut diikuti walk.
                                if let Some(cs) = row
                                    .get("table_row")
                                    .and_then(|tr| tr.get("cells"))
                                    .and_then(Value::as_array)
                                {
                                    for c in cs {
                                        children.extend(mention_page_ids(Some(c)));
                                    }
                                }
                                if !cells.iter().all(|c| c.trim().is_empty()) {
                                    lines.push(cells.join(" | "));
                                }
                            }
                        }
                    }
                }
                "link_to_page" => {
                    // Tautan eksplisit ke halaman lain (bukan child_page).
                    if let Some(id) = link_to_page_id(block) {
                        children.push(id);
                    }
                }
                "divider" | "image" | "video" | "file" | "pdf" | "bookmark" | "embed"
                | "equation" | "table_of_contents" | "breadcrumb" => { /* dilewati */ }
                _ => {
                    if let Some(text) = block_line(block) {
                        lines.push(text);
                    }
                    // @-mention halaman di dalam teks blok ikut diikuti walk.
                    children.extend(mention_page_ids(block.get(btype)));
                    // Satu level anak di-inline (mis. isi toggle/list bersarang).
                    if block
                        .get("has_children")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        if let Some(id) = block.get("id").and_then(Value::as_str) {
                            let sub = self.fetch_blocks(id).await?;
                            for sb in &sub {
                                let stype = sb.get("type").and_then(Value::as_str).unwrap_or("");
                                if stype == "child_page" {
                                    // ID halaman anak = ID blok anak itu
                                    // sendiri (lihat catatan di arm
                                    // `child_page` tingkat atas).
                                    if let Some(cid) = sb.get("id").and_then(Value::as_str) {
                                        children.push(cid.to_string());
                                    }
                                } else if let Some(text) = block_line(sb) {
                                    lines.push(format!("  {text}"));
                                }
                                // @-mention / link_to_page di anak satu level juga diikuti.
                                if let Some(id) = link_to_page_id(sb) {
                                    children.push(id);
                                }
                                children.extend(mention_page_ids(sb.get(stype)));
                            }
                        }
                    }
                }
            }
        }
        Ok((lines.join("\n"), children))
    }
}

/// Satu baris teks dari satu blok (None bila blok tidak punya teks).
/// Dipakai `flatten_blocks` untuk blok tingkat atas maupun anak satu level.
fn block_line(block: &Value) -> Option<String> {
    let btype = block.get("type").and_then(Value::as_str).unwrap_or("");
    match btype {
        "child_page" | "child_database" | "table" | "divider" | "image" | "video" | "file"
        | "pdf" | "bookmark" | "embed" | "equation" | "table_of_contents" | "breadcrumb" => None,
        _ => {
            let text = rich_text_of(block.get(btype));
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
    }
}

/// Gabung `rich_text` (array `{plain_text}`) menjadi satu string.
/// Dipakai untuk semua tipe blok yang menyimpan teks sebagai rich_text.
fn rich_text_of(v: Option<&Value>) -> String {
    let Some(v) = v else {
        return String::new();
    };
    // Bentuk 1: objek blok `{ "rich_text": [...] }`.
    if let Some(arr) = v.get("rich_text").and_then(Value::as_array) {
        return arr
            .iter()
            .filter_map(|t| t.get("plain_text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("");
    }
    // Bentuk 2: array rich_text langsung (sel tabel).
    if let Some(arr) = v.as_array() {
        return arr
            .iter()
            .filter_map(|t| t.get("plain_text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("");
    }
    // Bentuk 3: string polos.
    v.as_str().unwrap_or("").to_string()
}

/// ID halaman yang ditautkan dari satu nilai rich_text: @-mention halaman
/// (`{type:"mention", mention:{type:"page", page:{id}}}`) — BUKAN mention
/// user/tanggal/database. Menerima objek blok (`{rich_text:[...]}`) maupun
/// array rich_text langsung (sel tabel). Dipakai `flatten_blocks` supaya
/// walk subtree mengikuti tautan @-mention, bukan hanya `child_page`
/// (perbaikan A1/A5 2026-11-07: 15 halaman grup StrutStack hanya
/// tertaut lewat mention dari root dan tidak pernah masuk DB).
fn mention_page_ids(v: Option<&Value>) -> Vec<String> {
    let Some(v) = v else {
        return vec![];
    };
    let arr = if let Some(a) = v.get("rich_text").and_then(Value::as_array) {
        Some(a)
    } else {
        v.as_array()
    };
    let Some(arr) = arr else {
        return vec![];
    };
    let mut out = vec![];
    for t in arr {
        if t.get("type").and_then(Value::as_str) != Some("mention") {
            continue;
        }
        let m = t.get("mention");
        if m.and_then(|x| x.get("type")).and_then(Value::as_str) != Some("page") {
            continue;
        }
        if let Some(id) = m
            .and_then(|x| x.get("page"))
            .and_then(|p| p.get("id"))
            .and_then(Value::as_str)
        {
            out.push(id.to_string());
        }
    }
    out
}

/// ID halaman dari blok `link_to_page` (tautan eksplisit ke halaman lain).
/// `database_id` dikembalikan None — baris database dikecualikan (SA §2.3).
fn link_to_page_id(block: &Value) -> Option<String> {
    let inner = block.get("link_to_page")?;
    if inner.get("type").and_then(Value::as_str) != Some("page_id") {
        return None;
    }
    inner
        .get("page_id")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

impl NotionClient for LiveNotionClient {
    async fn search_changed(&self, since_ms: Option<i64>) -> Result<Vec<PageMeta>, NotionError> {
        // CATATAN (deviasi SA §3.1, dicatat di laporan): API search Notion
        // TIDAK mendukung filter `last_edited_time > cursor` di sisi server.
        // Yang dipakai: sort last_edited_time menurun + cutoff client-side —
        // berhenti saat halaman sudah lebih tua dari cursor.
        let mut out: Vec<PageMeta> = vec![];
        let mut cursor: Option<String> = None;
        loop {
            let mut body = json!({
                "page_size": 100,
                "sort": { "direction": "descending", "timestamp": "last_edited_time" },
                "filter": { "property": "object", "value": "page" },
            });
            if let Some(c) = &cursor {
                body["start_cursor"] = json!(c);
            }
            let resp = self.post_json("/search", body).await?;
            let mut stop = false;
            if let Some(results) = resp.get("results").and_then(Value::as_array) {
                for item in results {
                    let Some(id) = item.get("id").and_then(Value::as_str) else {
                        continue;
                    };
                    let edited = item
                        .get("last_edited_time")
                        .and_then(Value::as_str)
                        .and_then(parse_rfc3339_ms)
                        .unwrap_or(0);
                    if let Some(since) = since_ms {
                        if edited <= since {
                            stop = true;
                            break;
                        }
                    }
                    let parent_page_id = item.get("parent").and_then(|p| {
                        if p.get("type").and_then(Value::as_str) == Some("page_id") {
                            p.get("page_id")
                                .and_then(Value::as_str)
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    });
                    out.push(PageMeta {
                        page_id: id.to_string(),
                        title: Self::page_title(item),
                        parent_page_id,
                        last_edited_ms: edited,
                        url: item
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    });
                    if out.len() >= MAX_PAGES_PER_SOURCE {
                        stop = true;
                        break;
                    }
                }
            }
            if stop {
                break;
            }
            let has_more = resp
                .get("has_more")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            cursor = resp
                .get("next_cursor")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            if !has_more || cursor.is_none() {
                break;
            }
        }
        Ok(out)
    }

    async fn fetch_page(&self, page_id: &str) -> Result<PageDoc, NotionError> {
        let page = self.get_json(&format!("/pages/{page_id}")).await?;
        let title = Self::page_title(&page);
        let edited = page
            .get("last_edited_time")
            .and_then(Value::as_str)
            .and_then(parse_rfc3339_ms)
            .unwrap_or(0);
        let url = page
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let blocks = self.fetch_blocks(page_id).await?;
        let (text, children) = self.flatten_blocks(&blocks).await?;
        Ok(PageDoc {
            page_id: page_id.to_string(),
            title,
            parent_path: String::new(), // diisi engine (butuh peta judul grup)
            group_slug: String::new(),  // diisi engine
            content_text: text,
            notion_last_edited_ms: edited,
            notion_url: url,
            child_page_ids: children,
        })
    }

    async fn list_subtree(&self, root_page_id: &str) -> Result<Vec<PageDoc>, NotionError> {
        // Walk dari root lewat blok `child_page` PLUS tautan eksplisit
        // (@-mention halaman + `link_to_page` — SA §3.1 langkah 5,
        // diperluas 2026-11-07: halaman yang hanya ditautkan lewat mention
        // tidak pernah masuk DB). Tiap dokumen diambil SATU kali di sini;
        // engine menyimpan langsung tanpa fetch ulang.
        let mut out: Vec<PageDoc> = vec![];
        let mut queue: Vec<(String, usize)> = vec![(root_page_id.to_string(), 0)];
        let mut seen: HashMap<String, bool> = HashMap::new();
        while let Some((id, depth)) = queue.pop() {
            if seen.contains_key(&id) || out.len() >= MAX_PAGES_PER_SOURCE {
                continue;
            }
            seen.insert(id.clone(), true);
            let is_root = id == root_page_id;
            let doc = match self.fetch_page(&id).await {
                Ok(d) => d,
                // Tautan (@-mention / link_to_page) boleh menunjuk halaman
                // yang tidak dibagikan ke integrasi (404): lewati halaman
                // itu, jangan gagalkan seluruh backfill. Root sendiri
                // tetap gagal keras — bila root tidak terbaca, itu salah
                // konfigurasi yang harus terlihat.
                Err(NotionError::NotFound) if !is_root => continue,
                Err(e) => return Err(e),
            };
            let children = doc.child_page_ids.clone();
            out.push(doc);
            if depth < MAX_WALK_DEPTH {
                for child in children {
                    queue.push((child, depth + 1));
                }
            }
        }
        // Terbaru dulu (konsisten dengan search_changed).
        out.sort_by_key(|b| std::cmp::Reverse(b.notion_last_edited_ms));
        Ok(out)
    }
}

/// Enum dispatch agar handle tidak butuh `async_trait` (bukan dependensi).
enum Client {
    Live(LiveNotionClient),
    Mock(MockNotionClient),
}

impl NotionClient for Client {
    async fn search_changed(&self, since_ms: Option<i64>) -> Result<Vec<PageMeta>, NotionError> {
        match self {
            Client::Live(c) => c.search_changed(since_ms).await,
            Client::Mock(c) => c.search_changed(since_ms).await,
        }
    }

    async fn fetch_page(&self, page_id: &str) -> Result<PageDoc, NotionError> {
        match self {
            Client::Live(c) => c.fetch_page(page_id).await,
            Client::Mock(c) => c.fetch_page(page_id).await,
        }
    }

    async fn list_subtree(&self, root_page_id: &str) -> Result<Vec<PageDoc>, NotionError> {
        match self {
            Client::Live(c) => c.list_subtree(root_page_id).await,
            Client::Mock(c) => c.list_subtree(root_page_id).await,
        }
    }
}

// ------------------------------------------------------------------- handle

pub struct NotionHandle {
    cfg: NotionConfig,
    client: Client,
    /// Kunci serialisasi: scheduler dan trigger manual tidak pernah jalan
    /// bersamaan (pola `StocksHandle.lock`).
    lock: AsyncMutex<()>,
    db_path: String,
    stats: Arc<HttpStats>,
}

impl NotionHandle {
    pub fn from_env(data_dir: &str) -> Arc<Self> {
        let cfg = NotionConfig::from_env(data_dir);
        let stats = Arc::new(HttpStats::default());
        let client = if cfg.mode() == "live" {
            let http = reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent(concat!("rizal-agent-hq/", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            Client::Live(LiveNotionClient::new(
                http,
                cfg.notion_base.clone(),
                cfg.notion_version.clone(),
                cfg.notion_key.clone().unwrap_or_default(),
                stats.clone(),
            ))
        } else {
            Client::Mock(MockNotionClient::from_fixtures())
        };
        Arc::new(NotionHandle {
            cfg,
            client,
            lock: AsyncMutex::new(()),
            db_path: format!("{data_dir}/hq.db"),
            stats,
        })
    }

    pub fn mode(&self) -> &'static str {
        self.cfg.mode()
    }

    pub fn configured(&self) -> bool {
        self.cfg.configured()
    }

    pub fn interval_min(&self) -> u64 {
        self.cfg.sync_interval_min
    }

    pub fn rpm_used(&self) -> usize {
        self.stats.rpm()
    }

    /// Trigger satu sync run. Bila sync sedang berjalan, langsung balas
    /// `already_running` (tidak menunggu — handler HTTP tidak boleh
    /// menggantung selama satu backfill).
    pub async fn trigger(&self, manual: bool) -> Value {
        let Ok(_guard) = self.lock.try_lock() else {
            return json!({"synced": false, "started": false, "reason": "sync sedang berjalan"});
        };
        self.run_sync(manual).await
    }

    async fn run_sync(&self, manual: bool) -> Value {
        let started = Instant::now();
        let now = now_ms();
        let mode = self.mode();
        let sources = load_sources(&self.cfg.data_dir);
        if sources.iter().all(|s| !s.enabled) {
            return json!({"synced": false, "mode": mode, "reason": "tidak ada sumber dikonfigurasi — buat data/notion.json (lihat docs/NOTION.md)"});
        }
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                return json!({"synced": false, "mode": mode, "reason": format!("database tidak bisa dibuka: {e}")});
            }
        };
        let _ = conn.execute_batch("PRAGMA busy_timeout=5000;");
        ensure_notion_schema(&conn);
        // Daftarkan sumber terkonfigurasi (panel bisa menampilkannya bahkan
        // sebelum sync pertama).
        for s in &sources {
            let _ = conn.execute(
                "INSERT INTO notion_sources(id, root_page_id, title, enabled, include_subtree) VALUES(?1,?2,?3,?4,1)
                 ON CONFLICT(id) DO UPDATE SET root_page_id=excluded.root_page_id, title=excluded.title, enabled=excluded.enabled",
                rusqlite::params![s.id, s.root_page_id, s.title, s.enabled as i64],
            );
        }

        let mut checked = 0usize;
        let mut updated = 0usize;
        let mut unchanged = 0usize;
        let mut quarantined = 0usize;
        let mut failures: Vec<Value> = vec![];
        let mut per_source: Vec<Value> = vec![];
        // Halaman yang sudah diambil lewat pelengkap search (sekali per
        // run, dibagi semua sumber — search workspace-wide, bukan
        // per sumber).
        let mut search_fetched: HashSet<String> = HashSet::new();

        for source in sources.iter().filter(|s| s.enabled) {
            let cursor: Option<i64> =
                get_state(&conn, &format!("cursor:{}", source.id)).and_then(|v| v.parse().ok());
            let backfill_done =
                get_state(&conn, &format!("backfill:{}", source.id)).as_deref() == Some("1");
            let mut src_checked = 0usize;
            let mut src_updated = 0usize;
            let mut src_quarantined = 0usize;
            let mut src_failed = 0usize;
            let mut src_failures: Vec<Value> = vec![];
            let mut max_edited = cursor.unwrap_or(0);

            if !backfill_done {
                // Backfill pertama: full crawl subtree (checkpoint per halaman
                // — bila terinterupsi, lanjut dari cursor, bukan dari nol).
                match self.client.list_subtree(&source.root_page_id).await {
                    Ok(docs) => {
                        let mut walk_ids: HashSet<String> = HashSet::new();
                        for mut doc in docs {
                            walk_ids.insert(doc.page_id.clone());
                            src_checked += 1;
                            fill_parent_path(source, &mut doc);
                            match store_page(&conn, &doc, now) {
                                StoreOutcome::Updated => src_updated += 1,
                                StoreOutcome::Unchanged => {}
                                StoreOutcome::Quarantined => src_quarantined += 1,
                            }
                            max_edited = max_edited.max(doc.notion_last_edited_ms);
                            // Checkpoint per halaman.
                            set_state(
                                &conn,
                                &format!("cursor:{}", source.id),
                                &max_edited.to_string(),
                            );
                        }
                        // Pelengkap search (perbaikan A1 ronde 2, 2026-11-07):
                        // walk tidak bisa mencapai baris database (parent
                        // database_id — mis. R3-01/02/03 di tracker SQL) dan
                        // halaman di luar tree root (mis. sub-grup Obsidian —
                        // parent-nya halaman Obsidian, bukan salah satu root).
                        // search_changed(None) mengembalikan SEMUA halaman yang
                        // terlihat integrasi (paginasi penuh); yang belum ada
                        // di walk diambil lewat fetch_page. Halaman personal
                        // TIDAK ikut: integrasi tidak bisa membacanya (404
                        // object_not_found — terverifikasi live), jadi search
                        // tidak mengembalikannya; tidak perlu denylist.
                        // backfill:1 dicatat hanya bila walk DAN pelengkap
                        // search berhasil — bila search gagal, run berikutnya
                        // mengulang backfill (idempotent).
                        let mut search_ok = true;
                        match self.client.search_changed(None).await {
                            Ok(metas) => {
                                for meta in metas {
                                    if walk_ids.contains(&meta.page_id)
                                        || search_fetched.contains(&meta.page_id)
                                    {
                                        continue;
                                    }
                                    src_checked += 1;
                                    match self.client.fetch_page(&meta.page_id).await {
                                        Ok(mut doc) => {
                                            search_fetched.insert(meta.page_id.clone());
                                            fill_parent_path(source, &mut doc);
                                            match store_page(&conn, &doc, now) {
                                                StoreOutcome::Updated => src_updated += 1,
                                                StoreOutcome::Unchanged => {}
                                                StoreOutcome::Quarantined => {
                                                    src_quarantined += 1;
                                                }
                                            }
                                            max_edited = max_edited.max(doc.notion_last_edited_ms);
                                            set_state(
                                                &conn,
                                                &format!("cursor:{}", source.id),
                                                &max_edited.to_string(),
                                            );
                                        }
                                        // Halaman hasil search yang ternyata
                                        // tidak bisa dibaca (hak akses berubah
                                        // di tengah jalan): lewati.
                                        Err(NotionError::NotFound) => continue,
                                        Err(e) => {
                                            search_ok = false;
                                            src_failed += 1;
                                            src_failures.push(json!({
                                                "page_id": meta.page_id,
                                                "error": e.safe_msg(),
                                            }));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                search_ok = false;
                                let msg = e.safe_msg();
                                src_failures.push(json!({"source": source.id, "error": msg}));
                                set_state(&conn, &format!("last_error:{}", source.id), &msg);
                            }
                        }
                        if search_ok {
                            set_state(&conn, &format!("backfill:{}", source.id), "1");
                        }
                    }
                    Err(e) => {
                        let msg = e.safe_msg();
                        src_failures.push(json!({"source": source.id, "error": msg}));
                        set_state(&conn, &format!("last_error:{}", source.id), &msg);
                    }
                }
            } else {
                // Incremental: hanya halaman yang berubah sejak cursor.
                match self.client.search_changed(cursor).await {
                    Ok(metas) => {
                        // Peta judul untuk parent_path (dari hasil search +
                        // judul yang sudah tersimpan di DB).
                        let mut titles: HashMap<String, String> = HashMap::new();
                        for m in &metas {
                            titles.insert(m.page_id.clone(), m.title.clone());
                        }
                        if let Ok(mut stmt) = conn.prepare("SELECT page_id, title FROM notion_docs")
                        {
                            if let Ok(rows) = stmt.query_map([], |r| {
                                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                            }) {
                                for row in rows.flatten() {
                                    titles.entry(row.0).or_insert(row.1);
                                }
                            }
                        }
                        for meta in metas {
                            src_checked += 1;
                            match self.client.fetch_page(&meta.page_id).await {
                                Ok(mut doc) => {
                                    doc.group_slug = source.id.clone();
                                    doc.parent_path = parent_path_for(
                                        &source.title,
                                        meta.parent_page_id.as_deref(),
                                        &titles,
                                    );
                                    match store_page(&conn, &doc, now) {
                                        StoreOutcome::Updated => src_updated += 1,
                                        StoreOutcome::Unchanged => {}
                                        StoreOutcome::Quarantined => src_quarantined += 1,
                                    }
                                    max_edited = max_edited.max(doc.notion_last_edited_ms);
                                }
                                Err(e) => {
                                    src_failed += 1;
                                    src_failures.push(json!({
                                        "page_id": meta.page_id,
                                        "error": e.safe_msg(),
                                    }));
                                }
                            }
                        }
                        set_state(
                            &conn,
                            &format!("cursor:{}", source.id),
                            &max_edited.to_string(),
                        );
                    }
                    Err(e) => {
                        let msg = e.safe_msg();
                        src_failures.push(json!({"source": source.id, "error": msg}));
                        set_state(&conn, &format!("last_error:{}", source.id), &msg);
                    }
                }
            }

            if src_failures.is_empty() {
                set_state(&conn, &format!("failures:{}", source.id), "0");
            } else {
                set_state(
                    &conn,
                    &format!("failures:{}", source.id),
                    &src_failures.len().to_string(),
                );
            }
            checked += src_checked;
            updated += src_updated;
            quarantined += src_quarantined;
            unchanged += src_checked - src_updated - src_quarantined - src_failed;
            failures.extend(src_failures.iter().cloned());
            per_source.push(json!({
                "id": source.id,
                "pages_checked": src_checked,
                "pages_updated": src_updated,
                "pages_quarantined": src_quarantined,
                "failures": src_failures.len(),
            }));
        }

        // Bangun ulang indeks FTS sekali per sync, SEBELUM last_sync_at
        // dicatat — supaya pembaca status tidak mencari sebelum indeks siap.
        rebuild_fts(&conn, "notion_docs_fts");
        set_state(&conn, "last_sync_at", &now.to_string());
        json!({
            "synced": failures.is_empty(),
            "started": true,
            "manual": manual,
            "mode": mode,
            "pages_checked": checked,
            "pages_updated": updated,
            "pages_unchanged": unchanged,
            "pages_quarantined": quarantined,
            "failures": failures,
            "per_source": per_source,
            "took_ms": started.elapsed().as_millis() as u64,
        })
    }
}

/// Isi `parent_path` + `group_slug` dokumen. Dipakai jalur backfill
/// (parent tidak diketahui dari walk — pakai judul grup saja).
fn fill_parent_path(source: &Source, doc: &mut PageDoc) {
    doc.group_slug = source.id.clone();
    doc.parent_path = source.title.clone();
}

/// `"{grup} / {judul parent}"` atau `"{grup}"` bila parent tidak dikenal.
fn parent_path_for(
    group_title: &str,
    parent_id: Option<&str>,
    titles: &HashMap<String, String>,
) -> String {
    match parent_id.and_then(|id| titles.get(id)) {
        Some(t) => format!("{group_title} / {t}"),
        None => group_title.to_string(),
    }
}

/// Penjadwal sync: tick tiap interval (default 30 menit); tick pertama
/// langsung jalan (backfill di latar, tidak memblokir server).
pub fn spawn_scheduler(handle: Arc<NotionHandle>) {
    tokio::spawn(async move {
        let secs = handle.interval_min().max(5) * 60;
        let mut tick = tokio::time::interval(Duration::from_secs(secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            let r = handle.trigger(false).await;
            println!(
                "notion: sync mode={} checked={} updated={} quarantined={} failures={}",
                r.get("mode").and_then(Value::as_str).unwrap_or("?"),
                r.get("pages_checked").and_then(Value::as_u64).unwrap_or(0),
                r.get("pages_updated").and_then(Value::as_u64).unwrap_or(0),
                r.get("pages_quarantined")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                r.get("failures")
                    .and_then(Value::as_array)
                    .map(|a| a.len())
                    .unwrap_or(0),
            );
        }
    });
}

// ------------------------------------------------------------------- query

/// Pencarian gabungan FTS5 (memory_entries + notion_docs).
/// Urutan: trusted memory dulu, lalu synced doc, lalu candidate.
/// `q` kosong = daftar terbaru (urut waktu).
pub fn search_knowledge(
    conn: &Connection,
    q: &str,
    scope: Option<&str>,
    kind: Option<&str>,
    status: Option<&str>,
    limit: usize,
) -> Vec<Value> {
    let limit = limit.clamp(1, 50);
    let want_mem = kind.is_none_or(|k| k == "memory");
    let want_doc = kind.is_none_or(|k| k == "doc");
    let mut out: Vec<(u8, i64, Value)> = vec![];
    if want_mem {
        if q.trim().is_empty() {
            let sql = "SELECT id, kind, scope, title, body, status, evidence, created_by, created_at, promoted_by, promoted_at FROM memory_entries WHERE (?1 IS NULL OR status = ?1) AND (?2 IS NULL OR scope = ?2) ORDER BY created_at DESC LIMIT ?3";
            if let Ok(mut stmt) = conn.prepare(sql) {
                let rows = stmt.query_map(rusqlite::params![status, scope, limit as i64], |r| {
                    row_memory(r, None, 0)
                });
                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        let tier = if row["status"] == "trusted" { 0u8 } else { 2u8 };
                        out.push((tier, 0, row));
                    }
                }
            }
        } else {
            let esc = fts_escape(q);
            let sql = "SELECT m.id, m.kind, m.scope, m.title, m.body, m.status, m.evidence, m.created_by, m.created_at, m.promoted_by, m.promoted_at, snippet(memory_entries_fts, 1, '<b>', '</b>', '…', 24) AS snip, bm25(memory_entries_fts) AS rnk FROM memory_entries_fts JOIN memory_entries m ON m.rowid = memory_entries_fts.rowid WHERE memory_entries_fts MATCH ?1 AND (?2 IS NULL OR m.status = ?2) AND (?3 IS NULL OR m.scope = ?3) ORDER BY rnk LIMIT ?4";
            if let Ok(mut stmt) = conn.prepare(sql) {
                let rows =
                    stmt.query_map(rusqlite::params![esc, status, scope, limit as i64], |r| {
                        let mut v = row_memory(r, Some(11), 12)?;
                        v["score"] = json!(r.get::<_, f64>(12).unwrap_or(0.0));
                        Ok(v)
                    });
                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        let tier = if row["status"] == "trusted" { 0u8 } else { 2u8 };
                        let rank = (row["score"].as_f64().unwrap_or(0.0) * 1000.0) as i64;
                        out.push((tier, rank, row));
                    }
                }
            }
        }
    }
    if want_doc {
        if q.trim().is_empty() {
            let sql = "SELECT page_id, title, parent_path, group_slug, content_text, notion_url, notion_last_edited, synced_at, sync_status FROM notion_docs WHERE sync_status = COALESCE(?1, 'synced') AND (?2 IS NULL OR group_slug = ?2) ORDER BY synced_at DESC LIMIT ?3";
            if let Ok(mut stmt) = conn.prepare(sql) {
                let rows = stmt.query_map(rusqlite::params![status, scope, limit as i64], |r| {
                    row_doc(r, None, 0)
                });
                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        out.push((1u8, 0, row));
                    }
                }
            }
        } else {
            let esc = fts_escape(q);
            let sql = "SELECT n.page_id, n.title, n.parent_path, n.group_slug, n.content_text, n.notion_url, n.notion_last_edited, n.synced_at, n.sync_status, snippet(notion_docs_fts, 1, '<b>', '</b>', '…', 24) AS snip, bm25(notion_docs_fts) AS rnk FROM notion_docs_fts JOIN notion_docs n ON n.rowid = notion_docs_fts.rowid WHERE notion_docs_fts MATCH ?1 AND n.sync_status = COALESCE(?2, 'synced') AND (?3 IS NULL OR n.group_slug = ?3) ORDER BY rnk LIMIT ?4";
            if let Ok(mut stmt) = conn.prepare(sql) {
                let rows =
                    stmt.query_map(rusqlite::params![esc, status, scope, limit as i64], |r| {
                        let mut v = row_doc(r, Some(9), 10)?;
                        v["score"] = json!(r.get::<_, f64>(10).unwrap_or(0.0));
                        Ok(v)
                    });
                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        let rank = (row["score"].as_f64().unwrap_or(0.0) * 1000.0) as i64;
                        out.push((1u8, rank, row));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out.truncate(limit);
    out.into_iter().map(|(_, _, v)| v).collect()
}

fn row_memory(
    r: &rusqlite::Row,
    snip_col: Option<usize>,
    _rank_col: usize,
) -> rusqlite::Result<Value> {
    let body: String = r.get(4)?;
    let snippet = snip_col
        .and_then(|c| r.get::<_, Option<String>>(c).ok().flatten())
        .unwrap_or_else(|| body.chars().take(160).collect());
    Ok(json!({
        "kind": "memory",
        "id": r.get::<_, String>(0)?,
        "memory_kind": r.get::<_, String>(1)?,
        "scope": r.get::<_, String>(2)?,
        "title": r.get::<_, String>(3)?,
        "body": body,
        "snippet": snippet,
        "status": r.get::<_, String>(5)?,
        "evidence": r.get::<_, Option<String>>(6)?.and_then(|e| serde_json::from_str::<Value>(&e).ok()),
        "created_by": r.get::<_, String>(7)?,
        "created_at": r.get::<_, i64>(8)?,
        "promoted_by": r.get::<_, Option<String>>(9)?,
        "promoted_at": r.get::<_, Option<i64>>(10)?,
        "provenance": { "source": "memory" },
    }))
}

fn row_doc(
    r: &rusqlite::Row,
    snip_col: Option<usize>,
    _rank_col: usize,
) -> rusqlite::Result<Value> {
    let text: String = r.get(4)?;
    let snippet = snip_col
        .and_then(|c| r.get::<_, Option<String>>(c).ok().flatten())
        .unwrap_or_else(|| text.chars().take(160).collect());
    Ok(json!({
        "kind": "doc",
        "id": r.get::<_, String>(0)?,
        "title": r.get::<_, String>(1)?,
        "parent_path": r.get::<_, String>(2)?,
        "group": r.get::<_, String>(3)?,
        "snippet": snippet,
        "status": r.get::<_, String>(8)?,
        "provenance": {
            "source": "notion",
            "group": r.get::<_, String>(3)?,
            "url": r.get::<_, String>(5)?,
            "last_edited": r.get::<_, i64>(6)?,
            "synced_at": r.get::<_, i64>(7)?,
        },
    }))
}

/// Daftar halaman tersinkron (panel + audit cakupan sync).
pub fn list_pages(conn: &Connection, group: Option<&str>, status: Option<&str>) -> Vec<Value> {
    let mut rows = vec![];
    let sql = "SELECT page_id, title, parent_path, group_slug, word_count, notion_url, notion_last_edited, synced_at, sync_status, quarantine_note FROM notion_docs WHERE (?1 IS NULL OR group_slug = ?1) AND (?2 IS NULL OR sync_status = ?2) ORDER BY synced_at DESC LIMIT 200";
    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(iter) = stmt.query_map(rusqlite::params![group, status], |r| {
            Ok(json!({
                "page_id": r.get::<_, String>(0)?,
                "title": r.get::<_, String>(1)?,
                "parent_path": r.get::<_, String>(2)?,
                "group": r.get::<_, String>(3)?,
                "word_count": r.get::<_, i64>(4)?,
                "url": r.get::<_, String>(5)?,
                "last_edited": r.get::<_, i64>(6)?,
                "synced_at": r.get::<_, i64>(7)?,
                "status": r.get::<_, String>(8)?,
                "quarantine_note": r.get::<_, Option<String>>(9)?,
            }))
        }) {
            rows = iter.flatten().collect();
        }
    }
    rows
}

/// Satu dokumen lengkap (panel saat manusia membuka satu halaman).
pub fn get_page(conn: &Connection, page_id: &str) -> Option<Value> {
    let mut stmt = conn
        .prepare("SELECT page_id, title, parent_path, group_slug, content_text, word_count, notion_url, notion_last_edited, synced_at, sync_status, quarantine_note FROM notion_docs WHERE page_id = ?1")
        .ok()?;
    stmt.query_row(rusqlite::params![page_id], |r| {
        Ok(json!({
            "page_id": r.get::<_, String>(0)?,
            "title": r.get::<_, String>(1)?,
            "parent_path": r.get::<_, String>(2)?,
            "group": r.get::<_, String>(3)?,
            "content_text": r.get::<_, String>(4)?,
            "word_count": r.get::<_, i64>(5)?,
            "url": r.get::<_, String>(6)?,
            "last_edited": r.get::<_, i64>(7)?,
            "synced_at": r.get::<_, i64>(8)?,
            "status": r.get::<_, String>(9)?,
            "quarantine_note": r.get::<_, Option<String>>(10)?,
        }))
    })
    .ok()
}

/// Status sync untuk panel + `GET /api/knowledge/sync/status`.
pub fn sync_status(conn: &Connection, handle: &NotionHandle) -> Value {
    let last_sync: Option<i64> = get_state(conn, "last_sync_at").and_then(|v| v.parse().ok());
    let next_sync = last_sync.map(|t| t + (handle.interval_min() as i64) * 60_000);
    let mut per_source = vec![];
    if let Ok(mut stmt) = conn.prepare("SELECT id, title, enabled FROM notion_sources ORDER BY id")
    {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                let (id, title, enabled) = row;
                let pages: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM notion_docs WHERE group_slug = ?1",
                        rusqlite::params![id],
                        |r| r.get::<_, i64>(0),
                    )
                    .unwrap_or(0);
                per_source.push(json!({
                    "id": id,
                    "title": title,
                    "enabled": enabled == 1,
                    "pages": pages,
                    "failures": get_state(conn, &format!("failures:{id}")).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                    "last_error": get_state(conn, &format!("last_error:{id}")),
                }));
            }
        }
    }
    let total_pages: i64 = conn
        .query_row("SELECT COUNT(*) FROM notion_docs", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0);
    let quarantined: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notion_docs WHERE sync_status = 'quarantined'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0);
    json!({
        "configured": handle.configured(),
        "mode": handle.mode(),
        "interval_min": handle.interval_min(),
        "last_sync_at": last_sync,
        "next_sync_at": next_sync,
        "total_pages": total_pages,
        "quarantined": quarantined,
        "per_source": per_source,
        "rate": { "rpm_used": handle.rpm_used() },
    })
}

// ------------------------------------------------------------------- memori

const MEMORY_BODY_MAX_CHARS: usize = 2000;

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryInput {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub evidence: Option<Value>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub supersedes_id: Option<String>,
}

fn memory_event(conn: &Connection, entry_id: &str, action: &str, actor: &str, note: Option<&str>) {
    let _ = conn.execute(
        "INSERT INTO memory_events(at, entry_id, action, actor, note) VALUES(?1,?2,?3,?4,?5)",
        rusqlite::params![now_ms(), entry_id, action, actor, note],
    );
}

fn new_memory_id(conn: &Connection, now: i64) -> String {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM memory_entries", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0);
    format!("mem-{now}-{count}")
}

/// Buat entri memori. Menulis `trusted` langsung DITOLAK kecuali
/// `created_by` adalah `syahrizal` atau `melo` (ditegakkan di server).
/// Teks mengandung pola rahasia DITOLAK (bukan disimpan tersensor).
pub fn create_memory(conn: &Connection, input: &MemoryInput, now: i64) -> Result<Value, String> {
    let kind = input.kind.clone().unwrap_or_else(|| "semantic".into());
    if !["episodic", "semantic", "procedural"].contains(&kind.as_str()) {
        return Err("kind harus episodic | semantic | procedural.".into());
    }
    let scope = input.scope.clone().unwrap_or_else(|| "global".into());
    if scope.trim().is_empty() {
        return Err("scope tidak boleh kosong.".into());
    }
    let title = input.title.clone().unwrap_or_default();
    let body = input.body.clone().unwrap_or_default();
    if title.trim().is_empty() || body.trim().is_empty() {
        return Err("title dan body wajib diisi.".into());
    }
    if body.chars().count() > MEMORY_BODY_MAX_CHARS {
        return Err(format!(
            "body maks {MEMORY_BODY_MAX_CHARS} karakter (memori yang butuh lebih panjang adalah dokumen — simpan sebagai file dan rujuk)."
        ));
    }
    if contains_secret(&title) || contains_secret(&body) {
        return Err("teks mengandung pola rahasia — ditolak, bukan disimpan tersensor.".into());
    }
    let created_by = input.created_by.clone().unwrap_or_else(|| "melo".into());
    let want_trusted = input.status.as_deref() == Some("trusted");
    if want_trusted && created_by != "syahrizal" && created_by != "melo" {
        return Err(
            "menulis trusted langsung hanya boleh syahrizal / melo — yang lain menjadi candidate."
                .into(),
        );
    }
    let status = if want_trusted { "trusted" } else { "candidate" };
    let evidence = input.evidence.clone().unwrap_or(json!([]));
    let id = new_memory_id(conn, now);
    conn.execute(
        "INSERT INTO memory_entries(id, kind, scope, title, body, status, evidence, supersedes_id, valid_from, valid_to, created_by, created_at, promoted_by, promoted_at, use_count, last_used_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,NULL,?10,?11,NULL,NULL,0,NULL)",
        rusqlite::params![
            id,
            kind,
            scope,
            redact(&title),
            redact(&body),
            status,
            evidence.to_string(),
            input.supersedes_id,
            now,
            created_by,
            now,
        ],
    )
    .map_err(|e| format!("gagal simpan memori: {e}"))?;
    rebuild_fts(conn, "memory_entries_fts");
    memory_event(conn, &id, "proposed", &created_by, None);
    Ok(json!({"id": id, "status": status}))
}

fn memory_row(conn: &Connection, id: &str) -> Option<Value> {
    let mut stmt = conn
        .prepare("SELECT id, kind, scope, title, body, status, evidence, supersedes_id, valid_from, valid_to, created_by, created_at, promoted_by, promoted_at FROM memory_entries WHERE id = ?1")
        .ok()?;
    stmt.query_row(rusqlite::params![id], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "kind": r.get::<_, String>(1)?,
            "scope": r.get::<_, String>(2)?,
            "title": r.get::<_, String>(3)?,
            "body": r.get::<_, String>(4)?,
            "status": r.get::<_, String>(5)?,
            "evidence": r.get::<_, Option<String>>(6)?.and_then(|e| serde_json::from_str::<Value>(&e).ok()),
            "supersedes_id": r.get::<_, Option<String>>(7)?,
            "valid_from": r.get::<_, i64>(8)?,
            "valid_to": r.get::<_, Option<i64>>(9)?,
            "created_by": r.get::<_, String>(10)?,
            "created_at": r.get::<_, i64>(11)?,
            "promoted_by": r.get::<_, Option<String>>(12)?,
            "promoted_at": r.get::<_, Option<i64>>(13)?,
        }))
    })
    .ok()
}

/// Daftar memori (FTS5 bila ada q, daftar terbaru bila q kosong).
pub fn list_memory(
    conn: &Connection,
    q: &str,
    kind: Option<&str>,
    scope: Option<&str>,
    status: Option<&str>,
) -> Vec<Value> {
    let mut out = vec![];
    if q.trim().is_empty() {
        let sql = "SELECT id, kind, scope, title, body, status, evidence, created_by, created_at, promoted_by, promoted_at FROM memory_entries WHERE (?1 IS NULL OR kind = ?1) AND (?2 IS NULL OR scope = ?2) AND (?3 IS NULL OR status = ?3) ORDER BY created_at DESC LIMIT 50";
        if let Ok(mut stmt) = conn.prepare(sql) {
            let rows = stmt.query_map(rusqlite::params![kind, scope, status], |r| {
                row_memory(r, None, 0)
            });
            if let Ok(rows) = rows {
                out = rows.flatten().collect();
            }
        }
    } else {
        let esc = fts_escape(q);
        let sql = "SELECT m.id, m.kind, m.scope, m.title, m.body, m.status, m.evidence, m.created_by, m.created_at, m.promoted_by, m.promoted_at, snippet(memory_entries_fts, 1, '<b>', '</b>', '…', 24) AS snip, bm25(memory_entries_fts) AS rnk FROM memory_entries_fts JOIN memory_entries m ON m.rowid = memory_entries_fts.rowid WHERE memory_entries_fts MATCH ?1 AND (?2 IS NULL OR m.kind = ?2) AND (?3 IS NULL OR m.scope = ?3) AND (?4 IS NULL OR m.status = ?4) ORDER BY rnk LIMIT 50";
        if let Ok(mut stmt) = conn.prepare(sql) {
            let rows = stmt.query_map(rusqlite::params![esc, kind, scope, status], |r| {
                row_memory(r, Some(11), 12)
            });
            if let Ok(rows) = rows {
                out = rows.flatten().collect();
            }
        }
    }
    out
}

pub fn promote_memory(conn: &Connection, id: &str, actor: &str, now: i64) -> Result<Value, String> {
    let row = memory_row(conn, id).ok_or_else(|| "memori tidak ditemukan.".to_string())?;
    if row["status"] != "candidate" {
        return Err(format!(
            "hanya candidate yang bisa dipromosikan (status saat ini: {}).",
            row["status"].as_str().unwrap_or("?")
        ));
    }
    if actor != "syahrizal" && actor != "melo" {
        return Err("promosi hanya boleh syahrizal / melo.".into());
    }
    conn.execute(
        "UPDATE memory_entries SET status='trusted', promoted_by=?1, promoted_at=?2 WHERE id=?3",
        rusqlite::params![actor, now, id],
    )
    .map_err(|e| format!("gagal promosi: {e}"))?;
    memory_event(conn, id, "promoted", actor, None);
    Ok(json!({"id": id, "status": "trusted"}))
}

pub fn reject_memory(
    conn: &Connection,
    id: &str,
    actor: &str,
    reason: &str,
) -> Result<Value, String> {
    if reason.trim().is_empty() {
        return Err("alasan penolakan wajib diisi.".into());
    }
    let row = memory_row(conn, id).ok_or_else(|| "memori tidak ditemukan.".to_string())?;
    if row["status"] != "candidate" {
        return Err(format!(
            "hanya candidate yang bisa ditolak (status saat ini: {}).",
            row["status"].as_str().unwrap_or("?")
        ));
    }
    conn.execute(
        "UPDATE memory_entries SET status='rejected' WHERE id=?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("gagal menolak: {e}"))?;
    memory_event(conn, id, "rejected", actor, Some(reason));
    Ok(json!({"id": id, "status": "rejected"}))
}

pub fn correct_memory(
    conn: &Connection,
    id: &str,
    input: &MemoryInput,
    now: i64,
) -> Result<Value, String> {
    let old = memory_row(conn, id).ok_or_else(|| "memori tidak ditemukan.".to_string())?;
    if old["status"] == "rejected" {
        return Err("entri yang sudah rejected tidak bisa dikoreksi.".into());
    }
    let mut fixed = input.clone();
    fixed.kind = Some(
        input
            .kind
            .clone()
            .unwrap_or_else(|| old["kind"].as_str().unwrap_or("semantic").to_string()),
    );
    fixed.scope = Some(
        input
            .scope
            .clone()
            .unwrap_or_else(|| old["scope"].as_str().unwrap_or("global").to_string()),
    );
    fixed.title = Some(
        input
            .title
            .clone()
            .unwrap_or_else(|| old["title"].as_str().unwrap_or("").to_string()),
    );
    fixed.body = Some(
        input
            .body
            .clone()
            .unwrap_or_else(|| old["body"].as_str().unwrap_or("").to_string()),
    );
    fixed.evidence = Some(
        input
            .evidence
            .clone()
            .unwrap_or_else(|| old["evidence"].clone()),
    );
    fixed.status = Some("candidate".into());
    fixed.supersedes_id = Some(id.to_string());
    let created = create_memory(conn, &fixed, now)?;
    conn.execute(
        "UPDATE memory_entries SET status='superseded' WHERE id=?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("gagal menandai superseded: {e}"))?;
    memory_event(
        conn,
        id,
        "superseded",
        fixed.created_by.as_deref().unwrap_or("melo"),
        None,
    );
    Ok(created)
}

pub fn delete_memory(
    conn: &Connection,
    id: &str,
    actor: &str,
    reason: &str,
) -> Result<Value, String> {
    if reason.trim().is_empty() {
        return Err("alasan penghapusan wajib diisi (hanya untuk rahasia/PII).".into());
    }
    let row = memory_row(conn, id).ok_or_else(|| "memori tidak ditemukan.".to_string())?;
    let _ = row;
    conn.execute(
        "DELETE FROM memory_entries WHERE id=?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("gagal hapus: {e}"))?;
    rebuild_fts(conn, "memory_entries_fts");
    // Event dicatat TANPA isi entri (alasan saja).
    memory_event(conn, id, "deleted", actor, Some(reason));
    Ok(json!({"id": id, "deleted": true}))
}

/// Jalur promosi dari dokumen Notion: membuat KANDIDAT (bukan trusted).
/// Evidence dibangun otomatis dari dokumen (page_id + last_edited + URL).
pub fn promote_from_doc(
    conn: &Connection,
    page_id: &str,
    title: Option<String>,
    body: Option<String>,
    created_by: &str,
    now: i64,
) -> Result<Value, String> {
    let doc = get_page(conn, page_id).ok_or_else(|| "dokumen tidak ditemukan.".to_string())?;
    if doc["status"] == "quarantined" {
        return Err(
            "dokumen dikarantina (mengandung pola rahasia) — tidak bisa dipromosikan.".into(),
        );
    }
    let doc_title = doc["title"].as_str().unwrap_or("").to_string();
    let content = doc["content_text"].as_str().unwrap_or("").to_string();
    let group = doc["group"].as_str().unwrap_or("").to_string();
    let title = title.filter(|t| !t.trim().is_empty()).unwrap_or(doc_title);
    let body = body
        .filter(|b| !b.trim().is_empty())
        .unwrap_or_else(|| content.chars().take(MEMORY_BODY_MAX_CHARS).collect());
    let evidence = json!([{
        "type": "notion",
        "ref": page_id,
        "last_edited": doc["last_edited"].as_i64(),
        "url": doc["url"].as_str().unwrap_or(""),
    }]);
    let scope = if group == "hq" {
        "global".to_string()
    } else {
        format!("project:{group}")
    };
    let input = MemoryInput {
        kind: Some("semantic".into()),
        scope: Some(scope),
        title: Some(title),
        body: Some(body),
        status: Some("candidate".into()),
        evidence: Some(evidence),
        created_by: Some(created_by.to_string()),
        supersedes_id: None,
    };
    create_memory(conn, &input, now)
}

// ------------------------------------------------------------------- handler
// (axum — dipanggil dari main.rs; logika tinggal di fungsi di atas)

use crate::AppState;

#[derive(Deserialize)]
pub struct KnowledgeSearchQ {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct PagesQ {
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct PromoteBody {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
}

#[derive(Deserialize)]
pub struct ActorBody {
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

fn db_guard(st: &Arc<AppState>) -> Option<std::sync::MutexGuard<'_, Connection>> {
    st.db.lock().ok()
}

pub async fn get_knowledge_search(
    State(st): State<Arc<AppState>>,
    Query(q): Query<KnowledgeSearchQ>,
) -> Json<Value> {
    let Some(db) = db_guard(&st) else {
        return Json(json!({"results": [], "error": "database terkunci"}));
    };
    let results = search_knowledge(
        &db,
        q.q.as_deref().unwrap_or(""),
        q.scope.as_deref(),
        q.kind.as_deref(),
        q.status.as_deref(),
        q.limit.unwrap_or(10),
    );
    Json(json!({"results": results}))
}

pub async fn get_knowledge_pages(
    State(st): State<Arc<AppState>>,
    Query(q): Query<PagesQ>,
) -> Json<Value> {
    let Some(db) = db_guard(&st) else {
        return Json(json!({"pages": [], "error": "database terkunci"}));
    };
    Json(json!({"pages": list_pages(&db, q.group.as_deref(), q.status.as_deref())}))
}

pub async fn get_knowledge_page(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    let Some(db) = db_guard(&st) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database terkunci"})),
        );
    };
    match get_page(&db, &id) {
        Some(v) => (StatusCode::OK, Json(v)),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "dokumen tidak ditemukan."})),
        ),
    }
}

pub async fn get_knowledge_sync_status(State(st): State<Arc<AppState>>) -> Json<Value> {
    let Some(db) = db_guard(&st) else {
        return Json(
            json!({"configured": st.notion.configured(), "mode": st.notion.mode(), "error": "database terkunci"}),
        );
    };
    Json(sync_status(&db, &st.notion))
}

pub async fn post_knowledge_sync(State(st): State<Arc<AppState>>) -> Json<Value> {
    Json(st.notion.trigger(true).await)
}

pub async fn post_knowledge_promote(
    State(st): State<Arc<AppState>>,
    Json(body): Json<PromoteBody>,
) -> (StatusCode, Json<Value>) {
    let Some(page_id) = body.page_id.filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "page_id wajib diisi."})),
        );
    };
    let Some(db) = db_guard(&st) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database terkunci"})),
        );
    };
    let created_by = body.created_by.unwrap_or_else(|| "melo".into());
    match promote_from_doc(&db, &page_id, body.title, body.body, &created_by, now_ms()) {
        Ok(v) => (StatusCode::CREATED, Json(v)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

pub async fn get_memory(
    State(st): State<Arc<AppState>>,
    Query(q): Query<KnowledgeSearchQ>,
) -> Json<Value> {
    let Some(db) = db_guard(&st) else {
        return Json(json!({"entries": [], "error": "database terkunci"}));
    };
    Json(json!({"entries": list_memory(
        &db,
        q.q.as_deref().unwrap_or(""),
        q.kind.as_deref(),
        q.scope.as_deref(),
        q.status.as_deref(),
    )}))
}

pub async fn post_memory(
    State(st): State<Arc<AppState>>,
    Json(body): Json<MemoryInput>,
) -> (StatusCode, Json<Value>) {
    let Some(db) = db_guard(&st) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database terkunci"})),
        );
    };
    match create_memory(&db, &body, now_ms()) {
        Ok(v) => (StatusCode::CREATED, Json(v)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

pub async fn post_memory_promote(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ActorBody>,
) -> (StatusCode, Json<Value>) {
    let Some(db) = db_guard(&st) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database terkunci"})),
        );
    };
    let actor = body.actor.unwrap_or_else(|| "melo".into());
    match promote_memory(&db, &id, &actor, now_ms()) {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

pub async fn post_memory_reject(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ActorBody>,
) -> (StatusCode, Json<Value>) {
    let Some(db) = db_guard(&st) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database terkunci"})),
        );
    };
    let actor = body.actor.unwrap_or_else(|| "melo".into());
    match reject_memory(&db, &id, &actor, body.reason.as_deref().unwrap_or("")) {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

pub async fn post_memory_correct(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<MemoryInput>,
) -> (StatusCode, Json<Value>) {
    let Some(db) = db_guard(&st) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database terkunci"})),
        );
    };
    match correct_memory(&db, &id, &body, now_ms()) {
        Ok(v) => (StatusCode::CREATED, Json(v)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

pub async fn post_memory_delete(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ActorBody>,
) -> (StatusCode, Json<Value>) {
    let Some(db) = db_guard(&st) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database terkunci"})),
        );
    };
    let actor = body.actor.unwrap_or_else(|| "melo".into());
    match delete_memory(&db, &id, &actor, body.reason.as_deref().unwrap_or("")) {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

// --------------------------------------------------------------------- uji

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("db memori");
        ensure_notion_schema(&conn);
        conn
    }

    fn doc(page_id: &str, title: &str, text: &str, edited: i64) -> PageDoc {
        PageDoc {
            page_id: page_id.into(),
            title: title.into(),
            parent_path: "Rizal Agent HQ".into(),
            group_slug: "hq".into(),
            content_text: text.into(),
            notion_last_edited_ms: edited,
            notion_url: format!("https://app.notion.com/p/{page_id}"),
            child_page_ids: vec![],
        }
    }

    #[test]
    fn rfc3339_ke_epoch_ms() {
        // 2026-11-07T00:00:00Z = 1794009600000 (cek silang: 2026-11-07 07:00 WIB).
        assert_eq!(
            parse_rfc3339_ms("2026-11-07T00:00:00Z"),
            Some(1794009600000)
        );
        assert_eq!(
            parse_rfc3339_ms("2026-11-07T07:00:00+07:00"),
            Some(1794009600000)
        );
        assert_eq!(
            parse_rfc3339_ms("2026-11-06T15:30:00.123Z"),
            Some(1793979000123)
        );
        assert_eq!(parse_rfc3339_ms("bukan-tanggal"), None);
    }

    #[test]
    fn fts_escape_mengutip_token() {
        assert_eq!(fts_escape("both-pass 0065"), "\"both-pass\" AND \"0065\"");
        assert_eq!(fts_escape("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn simpan_hash_skip_tanpa_perubahan() {
        let conn = mem_conn();
        let d = doc("p1", "Judul", "isi dokumen tentang both-pass", 1000);
        assert_eq!(store_page(&conn, &d, 2000), StoreOutcome::Updated);
        // Simpan ulang isi sama: tidak ada tulis ulang, synced_at diperbarui.
        assert_eq!(store_page(&conn, &d, 3000), StoreOutcome::Unchanged);
        let synced: i64 = conn
            .query_row(
                "SELECT synced_at FROM notion_docs WHERE page_id='p1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(synced, 3000);
        // Edit isi: hash berubah -> Updated.
        let d2 = doc(
            "p1",
            "Judul",
            "isi dokumen tentang both-pass dan EODHD",
            4000,
        );
        assert_eq!(store_page(&conn, &d2, 5000), StoreOutcome::Updated);
        // Indeks FTS dibangun ulang oleh sync (di sini manual, karena test
        // memanggil store_page langsung, bukan run_sync).
        rebuild_fts(&conn, "notion_docs_fts");
        // Pencarian FTS5 menemukan kata baru, dan kata lama masih ada di judul/isi.
        let hits = search_knowledge(&conn, "EODHD", None, Some("doc"), None, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["id"], "p1");
    }

    #[test]
    fn simpan_jebakan_rahasia_dikarantina() {
        let conn = mem_conn();
        let d = doc(
            "trap",
            "Jebakan",
            "kunci cadangan: sk-ant-abc1234567890 jangan disebar",
            1000,
        );
        assert_eq!(store_page(&conn, &d, 2000), StoreOutcome::Quarantined);
        // Rebuild mengindeks judul jebakan juga — pencarian harus tetap
        // kosong karena filter default hanya menyajikan dokumen synced.
        rebuild_fts(&conn, "notion_docs_fts");
        let (status, text, note): (String, String, Option<String>) = conn
            .query_row(
                "SELECT sync_status, content_text, quarantine_note FROM notion_docs WHERE page_id='trap'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "quarantined");
        assert!(text.is_empty());
        assert!(note.is_some());
        // 0 hasil pencarian untuk pola rahasia itu.
        assert!(
            search_knowledge(&conn, "sk-ant-abc1234567890", None, Some("doc"), None, 10).is_empty()
        );
        assert!(search_knowledge(&conn, "Jebakan", None, Some("doc"), None, 10).is_empty());
    }

    #[test]
    fn urutan_pencarian_trusted_dulu() {
        let conn = mem_conn();
        store_page(
            &conn,
            &doc(
                "p1",
                "Dokumen EODHD",
                "EODHD adalah sumber harga saham",
                1000,
            ),
            2000,
        );
        let input = MemoryInput {
            kind: Some("semantic".into()),
            scope: Some("global".into()),
            title: Some("EODHD sumber harga".into()),
            body: Some("EODHD dipakai untuk harga saham IDX".into()),
            status: Some("trusted".into()),
            evidence: Some(json!([])),
            created_by: Some("melo".into()),
            supersedes_id: None,
        };
        create_memory(&conn, &input, 3000).unwrap();
        rebuild_fts(&conn, "notion_docs_fts");
        let hits = search_knowledge(&conn, "EODHD", None, None, None, 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0]["kind"], "memory"); // trusted dulu
        assert_eq!(hits[0]["status"], "trusted");
        assert_eq!(hits[1]["kind"], "doc");
    }

    #[test]
    fn tulis_trusted_ditolak_bila_bukan_manusia() {
        let conn = mem_conn();
        let input = MemoryInput {
            kind: Some("semantic".into()),
            scope: Some("global".into()),
            title: Some("X".into()),
            body: Some("isi".into()),
            status: Some("trusted".into()),
            evidence: Some(json!([])),
            created_by: Some("agent:dev".into()),
            supersedes_id: None,
        };
        assert!(create_memory(&conn, &input, 1000).is_err());
        // Tanpa status trusted -> candidate.
        let mut input2 = input.clone();
        input2.status = None;
        let out = create_memory(&conn, &input2, 1000).unwrap();
        assert_eq!(out["status"], "candidate");
    }

    #[test]
    fn tulis_rahasia_ditolak_bukan_disensor() {
        let conn = mem_conn();
        let input = MemoryInput {
            kind: Some("semantic".into()),
            scope: Some("global".into()),
            title: Some("Kunci".into()),
            body: Some("token=abc123".into()),
            status: None,
            evidence: None,
            created_by: Some("melo".into()),
            supersedes_id: None,
        };
        assert!(create_memory(&conn, &input, 1000).is_err());
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn promosi_candidate_ke_trusted_tercatat() {
        let conn = mem_conn();
        let input = MemoryInput {
            kind: Some("semantic".into()),
            scope: Some("global".into()),
            title: Some("Aturan both-pass".into()),
            body: Some("merge hanya bila review + QA lolos".into()),
            status: None,
            evidence: Some(json!([{"type": "notion", "ref": "p1"}])),
            created_by: Some("melo".into()),
            supersedes_id: None,
        };
        let out = create_memory(&conn, &input, 1000).unwrap();
        let id = out["id"].as_str().unwrap().to_string();
        let promoted = promote_memory(&conn, &id, "melo", 2000).unwrap();
        assert_eq!(promoted["status"], "trusted");
        let events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_events WHERE entry_id=?1 AND action='promoted'",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(events, 1);
    }

    #[test]
    fn tolak_butuh_alasan_dan_koreksi_membuat_rantai() {
        let conn = mem_conn();
        let mk = |title: &str| MemoryInput {
            kind: Some("semantic".into()),
            scope: Some("global".into()),
            title: Some(title.into()),
            body: Some("isi".into()),
            status: None,
            evidence: None,
            created_by: Some("melo".into()),
            supersedes_id: None,
        };
        let id1 = create_memory(&conn, &mk("Satu"), 1000).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(reject_memory(&conn, &id1, "melo", "").is_err());
        reject_memory(&conn, &id1, "melo", "tidak akurat").unwrap();
        let id2 = create_memory(&conn, &mk("Dua"), 3000).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let fixed = correct_memory(
            &conn,
            &id2,
            &MemoryInput {
                body: Some("isi yang dikoreksi".into()),
                ..mk("Dua (koreksi)")
            },
            4000,
        )
        .unwrap();
        let new_id = fixed["id"].as_str().unwrap().to_string();
        assert_ne!(new_id, id2);
        let old_status: String = conn
            .query_row(
                "SELECT status FROM memory_entries WHERE id=?1",
                rusqlite::params![id2],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old_status, "superseded");
        let supersedes: Option<String> = conn
            .query_row(
                "SELECT supersedes_id FROM memory_entries WHERE id=?1",
                rusqlite::params![new_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(supersedes.as_deref(), Some(id2.as_str()));
    }

    #[test]
    fn hapus_hanya_dengan_alasan_dan_tanpa_isi() {
        let conn = mem_conn();
        let input = MemoryInput {
            kind: Some("semantic".into()),
            scope: Some("global".into()),
            title: Some("Bocor".into()),
            body: Some("isi yang ternyata sensitif".into()),
            status: None,
            evidence: None,
            created_by: Some("melo".into()),
            supersedes_id: None,
        };
        let id = create_memory(&conn, &input, 1000).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(delete_memory(&conn, &id, "melo", "").is_err());
        delete_memory(&conn, &id, "melo", "mengandung PII").unwrap();
        assert!(memory_row(&conn, &id).is_none());
        let note: Option<String> = conn
            .query_row(
                "SELECT note FROM memory_events WHERE entry_id=?1 AND action='deleted'",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(note.as_deref(), Some("mengandung PII"));
    }

    #[test]
    fn promote_dari_dokumen_hanya_candidate() {
        let conn = mem_conn();
        store_page(
            &conn,
            &doc("p9", "Dokumen HQ", "isi dokumen hq tentang both-pass", 1000),
            2000,
        );
        let out = promote_from_doc(&conn, "p9", None, None, "melo", 3000).unwrap();
        assert_eq!(out["status"], "candidate");
        let row = memory_row(&conn, out["id"].as_str().unwrap()).unwrap();
        assert_eq!(row["scope"], "global");
        let ev = row["evidence"].clone();
        assert_eq!(ev[0]["type"], "notion");
        assert_eq!(ev[0]["ref"], "p9");
        // Dokumen karantina tidak bisa dipromosikan.
        store_page(
            &conn,
            &doc("trap2", "Jebakan", "gho_1234567890abcdefghij", 1000),
            2000,
        );
        assert!(promote_from_doc(&conn, "trap2", None, None, "melo", 3000).is_err());
    }

    #[tokio::test]
    async fn mock_sync_tiga_fixture_dua_synced_satu_karantina() {
        let client = MockNotionClient::from_fixtures();
        assert_eq!(client.pages.len(), 3);
        let metas = client.search_changed(None).await.unwrap();
        assert_eq!(metas.len(), 3);
        // Incremental: hanya fixture dengan last_edited > cursor.
        let newest = metas.iter().map(|m| m.last_edited_ms).max().unwrap();
        let recent = client.search_changed(Some(newest - 1)).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].last_edited_ms, newest);
    }

    #[test]
    fn mention_page_dari_rich_text() {
        let v = serde_json::json!({
            "rich_text": [
                {"type": "text", "plain_text": "Lihat "},
                {"type": "mention", "mention": {"type": "page", "page": {"id": "11111111-1111-4111-8111-111111111111"}}, "plain_text": "SQL"},
                {"type": "mention", "mention": {"type": "user", "user": {}}, "plain_text": "@Budi"},
                {"type": "mention", "mention": {"type": "database", "database": {"id": "44444444-4444-4444-8444-444444444444"}}, "plain_text": "DB"}
            ]
        });
        assert_eq!(
            mention_page_ids(Some(&v)),
            vec!["11111111-1111-4111-8111-111111111111".to_string()]
        );
        // Array rich_text langsung (sel tabel).
        let cell = serde_json::json!([
            {"type": "mention", "mention": {"type": "page", "page": {"id": "55555555-5555-4555-8555-555555555555"}}, "plain_text": "PRD"}
        ]);
        assert_eq!(
            mention_page_ids(Some(&cell)),
            vec!["55555555-5555-4555-8555-555555555555".to_string()]
        );
        assert!(mention_page_ids(None).is_empty());
    }

    #[test]
    fn link_to_page_hanya_page_id() {
        let page = serde_json::json!({
            "type": "link_to_page",
            "link_to_page": {"type": "page_id", "page_id": "33333333-3333-4333-8333-333333333333"}
        });
        assert_eq!(
            link_to_page_id(&page).as_deref(),
            Some("33333333-3333-4333-8333-333333333333")
        );
        let db = serde_json::json!({
            "type": "link_to_page",
            "link_to_page": {"type": "database_id", "database_id": "44444444-4444-4444-8444-444444444444"}
        });
        assert_eq!(link_to_page_id(&db), None);
    }

    #[tokio::test]
    async fn flatten_blocks_mengikuti_mention_dan_link() {
        let client = LiveNotionClient::new(
            reqwest::Client::new(),
            "http://127.0.0.1:9".to_string(),
            "2022-06-28".to_string(),
            "dummy".to_string(),
            std::sync::Arc::new(HttpStats {
                hits: std::sync::Mutex::new(std::collections::VecDeque::new()),
            }),
        );
        let blocks = serde_json::json!([
            {"id": "b1", "type": "paragraph", "paragraph": {"rich_text": [
                {"type": "text", "plain_text": "Lihat "},
                {"type": "mention", "mention": {"type": "page", "page": {"id": "11111111-1111-4111-8111-111111111111"}}, "plain_text": "SQL"}
            ]}},
            {"id": "22222222-2222-4222-8222-222222222222", "type": "child_page", "child_page": {"title": "Anak"}},
            {"id": "b3", "type": "link_to_page", "link_to_page": {"type": "page_id", "page_id": "33333333-3333-4333-8333-333333333333"}},
            {"id": "b4", "type": "paragraph", "paragraph": {"rich_text": [
                {"type": "mention", "mention": {"type": "user", "user": {}}, "plain_text": "@Budi"}
            ]}}
        ]);
        let arr = blocks.as_array().unwrap().to_vec();
        let (text, children) = client.flatten_blocks(&arr).await.unwrap();
        assert!(text.contains("Lihat SQL"), "{text}");
        assert_eq!(
            children,
            vec![
                "11111111-1111-4111-8111-111111111111".to_string(),
                "22222222-2222-4222-8222-222222222222".to_string(),
                "33333333-3333-4333-8333-333333333333".to_string(),
            ]
        );
    }
}
