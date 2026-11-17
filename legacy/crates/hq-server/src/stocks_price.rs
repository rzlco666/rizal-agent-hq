//! Harga saham live: EODHD (Live/EOD) lalu FMP, cache harian, dan penjadwal WIB.
//!
//! Aturan inti (docs/SAHAM.md):
//! - Kunci API hanya dari env `EODHD_API_KEY` / `FMP_API_KEY`; tidak pernah ditulis
//!   ke log, cache, respons API, atau dokumen. 401 hanya dicatat sebagai "token salah".
//! - Kuota EODHD (20/hari + bonus sekali pakai) diperiksa ke `/api/user` (0 panggilan)
//!   SEBELUM setiap pengambilan.
//! - Hari bursa Senin–Jumat. Live sekali sehari >= 15:00 WIB; bila Live gagal bukan
//!   karena kuota (jaringan/5xx), satu percobaan EOD >= 16:05 WIB di hari yang sama.
//! - Harga di `stocks.json` TIDAK PERNAH ditimpa; cache di `stocks_cache.json`.
//! - Rute: tanpa sufiks, `.JK`, atau `.US` -> EODHD; sufiks lain (mis. `.DE`) -> FMP.
//!   Saham US wajib pakai sufiks `.US` eksplisit (tanpa sufiks = IDX).

use hq_core::timefmt::{now_ms, wib_date, wib_hms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

/// Batas simbol per batch harian = batas kuota EODHD gratis per hari.
pub const DAILY_BATCH_CAP: usize = 20;
/// Awal jendela Live (menit WIB sejak tengah malam): 15:00.
pub const LIVE_WINDOW_MIN: u32 = 15 * 60;
/// Percobaan EOD pengganti Live: 16:05 WIB.
pub const EOD_RETRY_MIN: u32 = 16 * 60 + 5;
/// Siklus penjadwal.
const TICK_SECS: u64 = 300;
/// Jeda minimal antar permintaan EOD (detik).
const EOD_GAP_SECS: u64 = 1;

// ---------------------------------------------------------------- waktu (WIB)

/// Hari WIB (hari sejak epoch UTC; WIB = UTC+7 tetap, tanpa DST).
pub fn wib_day(epoch_ms: i64) -> i64 {
    (epoch_ms.div_euclid(1000) + 7 * 3600).div_euclid(86400)
}

/// Senin=0 … Minggu=6.
pub fn wib_weekday(epoch_ms: i64) -> i64 {
    (wib_day(epoch_ms) + 3).rem_euclid(7)
}

pub fn wib_is_weekend(epoch_ms: i64) -> bool {
    wib_weekday(epoch_ms) >= 5
}

/// Menit sejak 00:00 WIB.
pub fn wib_minute_of_day(epoch_ms: i64) -> u32 {
    let s = (epoch_ms.div_euclid(1000) + 7 * 3600).rem_euclid(86400);
    ((s / 3600) * 60 + (s % 3600) / 60) as u32
}

/// "2026-11-05T15:01:00+07:00".
pub fn iso_wib(epoch_ms: i64) -> String {
    format!("{}T{}+07:00", wib_date(epoch_ms), wib_hms(epoch_ms))
}

/// Tanggal + jam WIB -> epoch ms (kebalikan dari [`iso_wib`], dipakai uji).
pub fn epoch_ms_from_wib(date: &str, minute_of_day: u32) -> Option<i64> {
    let mut it = date.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d) * 86_400_000 + minute_of_day as i64 * 60_000 - 7 * 3_600_000)
}

/// Algoritma days-from-civil (Howard Hinnant) — tanpa dependensi tanggal.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

// ------------------------------------------------------------- berkas saham

fn default_currency() -> String {
    "IDR".into()
}

/// Satu baris `stocks.json` (harga file = kebenaran portofolio).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Stock {
    pub symbol: String,
    #[serde(default)]
    pub name: String,
    pub shares: f64,
    pub price: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Opsional: simbol prioritas saat jumlah simbol > 20 (docs/SAHAM.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<bool>,
}

/// Hasil membaca `stocks.json` — file tidak ada bukan error.
pub enum StocksFile {
    Missing,
    Invalid(String),
    Ok(Vec<Stock>),
}

pub fn read_stocks(path: &str) -> StocksFile {
    let txt = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return StocksFile::Missing,
    };
    match serde_json::from_str::<Vec<Stock>>(&txt) {
        Ok(v) => StocksFile::Ok(v),
        Err(e) => StocksFile::Invalid(format!("stocks.json tidak bisa dibaca: {e}")),
    }
}

// ------------------------------------------------------------------ rute API

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Eodhd,
    Fmp,
}

/// ("BBCA", None) / ("BBCA", Some("JK")) / ("AAPL", Some("US")).
pub fn symbol_parts(sym: &str) -> (&str, Option<&str>) {
    match sym.rsplit_once('.') {
        Some((base, suffix)) if !base.is_empty() && !suffix.is_empty() => (base, Some(suffix)),
        _ => (sym, None),
    }
}

/// Tanpa sufiks, `.JK`, atau `.US` -> EODHD; sufiks lain -> FMP.
/// EODHD paket gratis mencakup bursa US (terverifikasi: real-time + EOD
/// NVDA.US/TSM.US/TSLA.US kembali data); FMP paket saat ini 402.
pub fn route(sym: &str) -> Provider {
    match symbol_parts(sym).1 {
        None => Provider::Eodhd,
        Some(s) if s.eq_ignore_ascii_case("jk") => Provider::Eodhd,
        Some(s) if s.eq_ignore_ascii_case("us") => Provider::Eodhd,
        Some(_) => Provider::Fmp,
    }
}

/// Kode EODHD: BBCA -> BBCA.JK, BBCA.JK -> BBCA.JK, NVDA.US -> NVDA.US.
/// Tanpa sufiks = IDX (`.JK`); sufiks `.US` dipertahankan.
pub fn eodhd_code(sym: &str) -> String {
    let (base, suffix) = symbol_parts(sym);
    let base = base.to_ascii_uppercase();
    match suffix {
        Some(s) if s.eq_ignore_ascii_case("us") => format!("{base}.US"),
        _ => format!("{base}.JK"),
    }
}

// --------------------------------------------------------------- pilih simbol

/// Simbol yang diambil hari ini.
///
/// <= 20 simbol: semua. > 20: simbol `priority` (maks 20) selalu ikut, sisanya
/// bergilir satu batch per hari. Tanpa bendera prioritas, batch bergilir 20
/// simbol mulai dari urutan file pada hari ke-0.
pub fn select_daily(symbols: &[(String, bool)], day: i64) -> Vec<String> {
    if symbols.len() <= DAILY_BATCH_CAP {
        return symbols.iter().map(|(s, _)| s.clone()).collect();
    }
    let priority: Vec<String> = symbols
        .iter()
        .filter(|(_, p)| *p)
        .map(|(s, _)| s.clone())
        .take(DAILY_BATCH_CAP)
        .collect();
    let mut out = priority.clone();
    let capacity = DAILY_BATCH_CAP - out.len();
    if capacity == 0 {
        return out;
    }
    let pool: Vec<String> = symbols
        .iter()
        .filter(|(s, _)| !priority.contains(s))
        .map(|(s, _)| s.clone())
        .collect();
    if pool.is_empty() {
        return out;
    }
    let rounds = pool.len().div_ceil(capacity).max(1) as i64;
    let offset = day.rem_euclid(rounds) as usize * capacity;
    for i in 0..capacity.min(pool.len()) {
        out.push(pool[(offset + i) % pool.len()].clone());
    }
    out
}

// ------------------------------------------------------------------- penjadwal

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Tidak ada pengambilan; alasan ditampilkan/jelaskan.
    Skip(String),
    /// EODHD Live (batch, >= 15:00 WIB).
    Live,
    /// EODHD EOD per simbol (>= 16:05 WIB setelah Live gagal non-kuota).
    Eod,
}

pub struct GateCtx<'a> {
    pub now_ms: i64,
    pub last_fetch_date: Option<&'a str>,
    pub live_attempt_date: Option<&'a str>,
    pub live_failed_nonquota: bool,
    /// Trigger manual: boleh di luar jendela 15:00, tetap menaati akhir pekan,
    /// gerbang sekali-sehari, dan kuota.
    pub manual: bool,
}

/// Keputusan penjadwal (fungsi murni — semua aturan waktu di sini).
pub fn decide(c: &GateCtx) -> Action {
    let today = wib_date(c.now_ms);
    if wib_is_weekend(c.now_ms) {
        return Action::Skip(format!("akhir pekan ({today}) — pasar tutup"));
    }
    if c.last_fetch_date == Some(today.as_str()) {
        return Action::Skip("sudah diambil hari ini".into());
    }
    if c.live_attempt_date == Some(today.as_str()) {
        if c.live_failed_nonquota && wib_minute_of_day(c.now_ms) >= EOD_RETRY_MIN {
            return Action::Eod;
        }
        return Action::Skip("percobaan live hari ini sudah dilakukan".into());
    }
    if !c.manual && wib_minute_of_day(c.now_ms) < LIVE_WINDOW_MIN {
        return Action::Skip("sebelum 15:00 WIB".into());
    }
    Action::Live
}

// ------------------------------------------------------------------ kuota

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quota {
    Proceed,
    /// Sisa kuota < kebutuhan: ambil `n` simbol prioritas teratas saja.
    Limited(usize),
    /// Sisa 0: tunda ke hari berikutnya.
    Defer,
}

/// `remaining = (daily_limit - used) + extra_limit`.
pub fn quota_decision(api_requests: i64, daily_limit: i64, extra_limit: i64, need: usize) -> Quota {
    let remaining = (daily_limit - api_requests)
        .max(0)
        .saturating_add(extra_limit.max(0));
    if remaining <= 0 {
        return Quota::Defer;
    }
    let remaining = remaining as usize;
    if remaining >= need {
        Quota::Proceed
    } else {
        Quota::Limited(remaining)
    }
}

// --------------------------------------------------------- normalisasi balasan

/// Satu kutipan hasil normalisasi.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedQuote {
    pub symbol: String,
    pub price: f64,
    pub previous_close: Option<f64>,
    /// Tanggal bursa "YYYY-MM-DD" (WIB untuk Live).
    pub asof: String,
}

#[derive(Debug, Default)]
pub struct LiveBatch {
    pub quotes: Vec<FetchedQuote>,
    /// (kode/simbol, pesan) — simbol buruk, tetap memakan 1 panggilan kuota.
    pub bad: Vec<String>,
}

fn value_price(obj: &serde_json::Map<String, Value>) -> Option<f64> {
    obj.get("close")
        .or_else(|| obj.get("price"))
        .and_then(Value::as_f64)
}

/// EODHD real-time: satu simbol = satu objek; batch (`s=`) = objek per kode.
/// Objek error (`{"code":"X","error":"..."}`) masuk `bad`, bukan kutipan.
pub fn normalize_live(
    v: &Value,
    code_to_symbol: &BTreeMap<String, String>,
    fallback_asof: &str,
) -> LiveBatch {
    let mut out = LiveBatch::default();
    match v {
        Value::Object(m) if m.contains_key("code") => {
            push_live(&mut out, m, "", code_to_symbol, fallback_asof)
        }
        Value::Object(m) => {
            for (key, x) in m {
                if let Some(obj) = x.as_object() {
                    push_live(&mut out, obj, key, code_to_symbol, fallback_asof);
                }
            }
        }
        _ => {}
    }
    out
}

fn push_live(
    out: &mut LiveBatch,
    obj: &serde_json::Map<String, Value>,
    key: &str,
    code_to_symbol: &BTreeMap<String, String>,
    fallback_asof: &str,
) {
    let code = obj
        .get("code")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(key);
    let price = value_price(obj);
    if obj.contains_key("error") || price.is_none() {
        out.bad.push(if code.is_empty() {
            "batch".to_string()
        } else {
            code.to_string()
        });
        return;
    }
    let symbol = code_to_symbol
        .get(&code.to_ascii_uppercase())
        .cloned()
        .unwrap_or_else(|| symbol_parts(code).0.to_string());
    let ts_ms = obj.get("timestamp").and_then(Value::as_i64).map(|t| {
        if t > 10_000_000_000 {
            t
        } else {
            t * 1000
        }
    });
    let asof = ts_ms
        .map(wib_date)
        .unwrap_or_else(|| fallback_asof.to_string());
    out.quotes.push(FetchedQuote {
        symbol,
        price: price.unwrap_or(0.0),
        previous_close: obj.get("previousClose").and_then(Value::as_f64),
        asof,
    });
}

/// EODHD EOD: array harian; elemen TERAKHIR = penutupan terbaru.
#[derive(Debug, Clone, PartialEq)]
pub struct EodPoint {
    pub date: String,
    pub price: f64,
}

pub fn pick_last_eod(v: &Value) -> Option<EodPoint> {
    let arr = v.as_array()?;
    let last = arr.last()?.as_object()?;
    let date = last.get("date")?.as_str()?.to_string();
    let price = last.get("close").and_then(Value::as_f64)?;
    Some(EodPoint { date, price })
}

// -------------------------------------------------------------------- cache

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CachedQuote {
    pub symbol: String,
    pub price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_close: Option<f64>,
    /// Tanggal bursa "YYYY-MM-DD".
    pub asof: String,
    /// ISO WIB saat diambil.
    pub fetched_at: String,
    /// "EODHD-Live" | "EODHD-EOD" | "FMP".
    pub source: String,
    /// Tanggal WIB pengambilan berhasil (dasar label "data terakhir"/stale).
    pub last_fetch_wib_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StocksCache {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quotes_updated_at: Option<String>,
    /// Gerbang sekali-sehari: diisi setelah pengambilan tuntas (sukses/kutipan
    /// habis/token salah), tidak diisi saat gagal non-kuota agar EOD 16:05 bisa jalan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_fetch_wib_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_attempt_wib_date: Option<String>,
    #[serde(default)]
    pub live_attempt_failed_nonquota: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls_used_today: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls_used_date: Option<String>,
    #[serde(default)]
    pub symbols: BTreeMap<String, CachedQuote>,
}

pub fn cache_path(data_dir: &str) -> String {
    format!("{data_dir}/stocks_cache.json")
}

pub fn load_cache(data_dir: &str) -> StocksCache {
    std::fs::read_to_string(cache_path(data_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Tulis cache secara atomik (tmp + rename) supaya file tidak pernah setengah jadi.
pub fn save_cache(data_dir: &str, cache: &StocksCache) -> Result<(), String> {
    let path = cache_path(data_dir);
    let tmp = format!("{path}.tmp");
    let body = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------- konfigurasi

pub struct StocksConfig {
    pub data_dir: String,
    eodhd_key: Option<String>,
    fmp_key: Option<String>,
    eodhd_base: String,
    fmp_base: String,
    fake_date: Option<String>,
    fake_time: Option<String>,
}

/// Debug tanpa kunci API (kunci tidak boleh ikut tercetak).
impl std::fmt::Debug for StocksConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StocksConfig")
            .field("data_dir", &self.data_dir)
            .field("eodhd_key", &self.eodhd_key.as_ref().map(|_| "<dari env>"))
            .field("fmp_key", &self.fmp_key.as_ref().map(|_| "<dari env>"))
            .field("eodhd_base", &self.eodhd_base)
            .field("fmp_base", &self.fmp_base)
            .field("fake_date", &self.fake_date)
            .field("fake_time", &self.fake_time)
            .finish()
    }
}

fn env_key(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl StocksConfig {
    pub fn from_env(data_dir: &str) -> Self {
        let base = |name: &str, default: &str| {
            std::env::var(name)
                .ok()
                .map(|v| v.trim().trim_end_matches('/').to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| default.to_string())
        };
        StocksConfig {
            data_dir: data_dir.to_string(),
            eodhd_key: env_key("EODHD_API_KEY"),
            fmp_key: env_key("FMP_API_KEY"),
            eodhd_base: base("EODHD_BASE_URL", "https://eodhd.com"),
            fmp_base: base("FMP_BASE_URL", "https://financialmodelingprep.com"),
            fake_date: env_key("HQ_STOCKS_FAKE_WIB_DATE"),
            fake_time: env_key("HQ_STOCKS_FAKE_WIB_TIME"),
        }
    }
}

// ------------------------------------------------------------------- kegagalan

#[derive(Debug, Clone, PartialEq)]
enum Fail {
    NoKey,
    /// 402 (EODHD) / 403 (FMP): kuota habis atau fitur berbayar.
    Quota,
    /// 401.
    Auth,
    RateLimited(Option<u64>),
    /// 404 / simbol tidak dikenal (tetap memakan 1 panggilan).
    Ticker,
    Network(&'static str),
    Server(u16),
    BadBody,
}

impl Fail {
    /// Pesan aman untuk log/JSON — tidak pernah memuat URL atau kunci.
    fn safe_msg(&self) -> String {
        match self {
            Fail::NoKey => "API key belum diatur".into(),
            Fail::Quota => "kuota API habis/ditolak (402/403)".into(),
            Fail::Auth => "token salah (401)".into(),
            Fail::RateLimited(_) => "terlalu banyak permintaan (429)".into(),
            Fail::Ticker => "simbol tidak dikenal (404)".into(),
            Fail::Network(m) => format!("jaringan: {m}"),
            Fail::Server(s) => format!("server membalas {s}"),
            Fail::BadBody => "balasan tidak dikenal".into(),
        }
    }

    /// Gagal non-kuota = boleh dicoba ulang lewat EOD di hari yang sama.
    fn nonquota(&self) -> bool {
        matches!(self, Fail::Network(_) | Fail::Server(_))
    }
}

/// Pesan error reqwest tanpa URL (URL memuat api_token).
fn safe_net_err(e: &reqwest::Error) -> Fail {
    if e.is_timeout() {
        Fail::Network("timeout")
    } else if e.is_connect() {
        Fail::Network("koneksi gagal")
    } else {
        Fail::Network("permintaan gagal")
    }
}

fn status_fail(status: u16, retry: Option<u64>) -> Option<Fail> {
    match status {
        200..=299 => None,
        401 => Some(Fail::Auth),
        402 | 403 => Some(Fail::Quota),
        404 => Some(Fail::Ticker),
        429 => Some(Fail::RateLimited(retry)),
        s => Some(Fail::Server(s)),
    }
}

// ------------------------------------------------------------------- handle

struct Guard {
    used: i64,
    limit: i64,
    extra: i64,
}

pub struct StocksHandle {
    cfg: StocksConfig,
    http: reqwest::Client,
    lock: tokio::sync::Mutex<()>,
}

impl StocksHandle {
    pub fn from_env(data_dir: &str) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("rizal-agent-hq/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Arc::new(StocksHandle {
            cfg: StocksConfig::from_env(data_dir),
            http,
            lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Jam WIB efektif; tanggal palsu hanya untuk uji (`HQ_STOCKS_FAKE_WIB_DATE`).
    pub fn now_ms(&self) -> i64 {
        match &self.cfg.fake_date {
            Some(d) => {
                let minute = self
                    .cfg
                    .fake_time
                    .as_deref()
                    .and_then(parse_hhmm)
                    .unwrap_or(15 * 60 + 5);
                epoch_ms_from_wib(d, minute).unwrap_or_else(now_ms)
            }
            None => now_ms(),
        }
    }

    pub fn key_configured(&self) -> bool {
        self.cfg.eodhd_key.is_some() || self.cfg.fmp_key.is_some()
    }

    /// `quotes` + `quotes_updated_at` untuk GET /api/stocks.
    pub fn quotes_view(&self) -> (Value, Option<String>) {
        let cache = load_cache(&self.cfg.data_dir);
        let today = wib_date(self.now_ms());
        let mut map = serde_json::Map::new();
        for (sym, q) in &cache.symbols {
            map.insert(
                sym.clone(),
                json!({
                    "price": q.price,
                    "asof": q.asof,
                    "fetched_at": q.fetched_at,
                    "source": q.source,
                    "stale": q.last_fetch_wib_date != today,
                }),
            );
        }
        (Value::Object(map), cache.quotes_updated_at.clone())
    }

    /// POST /api/stocks/refresh: hormati gerbang & akhir pekan; kunci serialisasi
    /// mencegah penjadwal dan trigger manual menembak bersamaan.
    pub async fn trigger(&self, manual: bool) -> Value {
        let _guard = self.lock.lock().await;
        self.run(manual).await
    }

    async fn run(&self, manual: bool) -> Value {
        let now = self.now_ms();
        let today = wib_date(now);
        let stocks = match read_stocks(&format!("{}/stocks.json", self.cfg.data_dir)) {
            StocksFile::Ok(v) => v,
            StocksFile::Missing => {
                return json!({"refreshed": false, "reason": "stocks.json tidak ada"})
            }
            StocksFile::Invalid(e) => return json!({"refreshed": false, "reason": e}),
        };
        if stocks.is_empty() {
            return json!({"refreshed": false, "reason": "tidak ada simbol di stocks.json"});
        }
        if !self.key_configured() {
            return json!({
                "refreshed": false,
                "reason": "harga live belum dikonfigurasi (EODHD_API_KEY/FMP_API_KEY kosong)"
            });
        }

        let mut cache = load_cache(&self.cfg.data_dir);
        let action = decide(&GateCtx {
            now_ms: now,
            last_fetch_date: cache.last_fetch_wib_date.as_deref(),
            live_attempt_date: cache.live_attempt_wib_date.as_deref(),
            live_failed_nonquota: cache.live_attempt_failed_nonquota,
            manual,
        });
        let eod = match action {
            Action::Skip(reason) => return json!({"refreshed": false, "reason": reason}),
            Action::Eod => true,
            Action::Live => false,
        };

        let entries: Vec<(String, bool)> = stocks
            .iter()
            .map(|s| (s.symbol.clone(), s.priority.unwrap_or(false)))
            .collect();
        let selected = select_daily(&entries, wib_day(now));
        let mut eodhd_syms: Vec<String> = selected
            .iter()
            .filter(|s| route(s) == Provider::Eodhd)
            .cloned()
            .collect();
        let fmp_syms: Vec<String> = selected
            .iter()
            .filter(|s| route(s) == Provider::Fmp)
            .cloned()
            .collect();

        let mut updated = 0usize;
        let mut notes: Vec<String> = vec![];
        let mut terminal = false;
        let mut live_failed_nonquota = false;

        // --- L1: EODHD (Live, atau EOD bila Live gagal non-kuota) ---
        if !eodhd_syms.is_empty() {
            if self.cfg.eodhd_key.is_none() {
                notes.push("EODHD_API_KEY kosong — simbol EODHD dilewati".into());
            } else {
                match self.quota_guard().await {
                    Ok(g) => {
                        cache.calls_used_today = Some(g.used);
                        cache.calls_used_date = Some(today.clone());
                        let decision = quota_decision(g.used, g.limit, g.extra, eodhd_syms.len());
                        if g.extra > 0 && g.used >= g.limit {
                            println!(
                                "saham: memakai bonus kuota EODHD ({} tersisa) — buffer sekali pakai",
                                g.extra
                            );
                        }
                        match decision {
                            Quota::Defer => {
                                notes.push(
                                    "kuota EODHD habis — cache dipakai, coba lagi besok".into(),
                                );
                                terminal = true;
                            }
                            Quota::Limited(n) => {
                                notes.push(format!("kuota tersisa {n} simbol — sisanya ditunda"));
                                eodhd_syms.truncate(n);
                                self.fetch_eodhd(
                                    &eodhd_syms,
                                    eod,
                                    now,
                                    &today,
                                    &mut cache,
                                    &mut updated,
                                    &mut notes,
                                    &mut terminal,
                                    &mut live_failed_nonquota,
                                )
                                .await;
                            }
                            Quota::Proceed => {
                                self.fetch_eodhd(
                                    &eodhd_syms,
                                    eod,
                                    now,
                                    &today,
                                    &mut cache,
                                    &mut updated,
                                    &mut notes,
                                    &mut terminal,
                                    &mut live_failed_nonquota,
                                )
                                .await;
                            }
                        }
                    }
                    Err(f) => {
                        // Gerbang kuota wajib; tanpa angka kuota jangan menembak buta.
                        notes.push(format!("pemeriksaan kuota: {}", f.safe_msg()));
                        if f.nonquota() {
                            live_failed_nonquota = true;
                        } else {
                            terminal = true;
                        }
                    }
                }
            }
        }

        // --- L2: FMP (khusus simbol non-EODHD) ---
        if !fmp_syms.is_empty() {
            if self.cfg.fmp_key.is_none() {
                notes.push("FMP_API_KEY kosong — simbol FMP dilewati".into());
            } else {
                for (i, sym) in fmp_syms.iter().enumerate() {
                    if i > 0 {
                        tokio::time::sleep(Duration::from_secs(EOD_GAP_SECS)).await;
                    }
                    match self.fmp_quote(sym).await {
                        Ok(q) => {
                            cache
                                .symbols
                                .insert(sym.clone(), cache_quote(q, now, &today, "FMP"));
                            updated += 1;
                        }
                        Err(f) => notes.push(format!("FMP {sym}: {}", f.safe_msg())),
                    }
                }
            }
        }

        cache.live_attempt_wib_date = Some(today.clone());
        cache.live_attempt_failed_nonquota = live_failed_nonquota;
        // Percobaan EOD selalu menutup hari itu (tidak ada percobaan tanpa henti).
        if terminal || eod || !live_failed_nonquota {
            cache.last_fetch_wib_date = Some(today.clone());
        }
        if updated > 0 {
            cache.quotes_updated_at = Some(iso_wib(now));
        }
        let save_err = save_cache(&self.cfg.data_dir, &cache).err();

        let refreshed = updated > 0;
        let mut reason = if refreshed {
            format!("{updated} simbol diperbarui")
        } else {
            "tidak ada harga baru — cache lama dipakai".to_string()
        };
        if let Some(e) = save_err {
            notes.push(format!("cache gagal ditulis: {e}"));
        }
        if !notes.is_empty() {
            reason = format!("{reason}; {}", notes.join("; "));
        }
        println!("saham: {reason}");
        json!({"refreshed": refreshed, "reason": reason})
    }

    #[allow(clippy::too_many_arguments)]
    async fn fetch_eodhd(
        &self,
        syms: &[String],
        eod: bool,
        now: i64,
        today: &str,
        cache: &mut StocksCache,
        updated: &mut usize,
        notes: &mut Vec<String>,
        terminal: &mut bool,
        live_failed_nonquota: &mut bool,
    ) {
        if syms.is_empty() {
            return;
        }
        if !eod {
            match self.eodhd_live(syms).await {
                Ok(batch) => {
                    let got = batch.quotes.len();
                    for q in batch.quotes {
                        cache
                            .symbols
                            .insert(q.symbol.clone(), cache_quote(q, now, today, "EODHD-Live"));
                        *updated += 1;
                    }
                    for b in batch.bad {
                        notes.push(format!("EODHD: {b} tidak dikenal"));
                    }
                    if got == 0 {
                        *live_failed_nonquota = true;
                    }
                }
                Err(f) => {
                    notes.push(format!("EODHD Live: {}", f.safe_msg()));
                    if f.nonquota() {
                        *live_failed_nonquota = true;
                    } else {
                        // 402/401/404: percobaan ulang hari ini tidak berguna.
                        *terminal = true;
                    }
                }
            }
            return;
        }
        let source = "EODHD-EOD";
        for (i, sym) in syms.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(Duration::from_secs(EOD_GAP_SECS)).await;
            }
            match self.eodhd_eod(sym, now).await {
                Ok(point) => {
                    cache.symbols.insert(
                        sym.clone(),
                        CachedQuote {
                            symbol: sym.clone(),
                            price: point.price,
                            previous_close: None,
                            asof: point.date,
                            fetched_at: iso_wib(now),
                            source: source.into(),
                            last_fetch_wib_date: today.to_string(),
                        },
                    );
                    *updated += 1;
                }
                Err(f) => {
                    notes.push(format!("EODHD EOD {sym}: {}", f.safe_msg()));
                    match f {
                        Fail::Quota | Fail::Auth => *terminal = true,
                        Fail::Network(_) | Fail::Server(_) => *live_failed_nonquota = true,
                        _ => {}
                    }
                }
            }
        }
    }

    // --- HTTP ---

    async fn fetch_text(&self, url: &str) -> Result<(u16, Option<u64>, String), Fail> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| safe_net_err(&e))?;
        let status = resp.status().as_u16();
        let retry = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());
        let text = resp.text().await.map_err(|e| safe_net_err(&e))?;
        Ok((status, retry, text))
    }

    /// GET dengan satu percobaan ulang untuk 429 (Retry-After, <= 60 detik).
    async fn get_checked(&self, url: &str) -> Result<String, Fail> {
        for attempt in 0..2 {
            let (status, retry, text) = self.fetch_text(url).await?;
            match status_fail(status, retry) {
                None => return Ok(text),
                Some(Fail::RateLimited(r)) if attempt == 0 => {
                    let wait = r.unwrap_or(2).clamp(1, 60);
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                }
                Some(f) => return Err(f),
            }
        }
        Err(Fail::RateLimited(None))
    }

    async fn quota_guard(&self) -> Result<Guard, Fail> {
        let key = self.cfg.eodhd_key.as_ref().ok_or(Fail::NoKey)?;
        let url = format!(
            "{}/api/user?api_token={}&fmt=json",
            self.cfg.eodhd_base, key
        );
        let text = self.get_checked(&url).await?;
        let v: Value = serde_json::from_str(&text).map_err(|_| Fail::BadBody)?;
        let pick = |k: &str| v.get(k).and_then(Value::as_i64);
        match (pick("apiRequests"), pick("dailyRateLimit")) {
            (Some(used), Some(limit)) => Ok(Guard {
                used,
                limit,
                extra: pick("extraLimit").unwrap_or(0),
            }),
            _ => Err(Fail::BadBody),
        }
    }

    async fn eodhd_live(&self, syms: &[String]) -> Result<LiveBatch, Fail> {
        let key = self.cfg.eodhd_key.as_ref().ok_or(Fail::NoKey)?;
        let codes: Vec<String> = syms.iter().map(|s| eodhd_code(s)).collect();
        let first = codes.first().ok_or(Fail::BadBody)?;
        let mut url = format!(
            "{}/api/real-time/{}?api_token={}&fmt=json",
            self.cfg.eodhd_base, first, key
        );
        if codes.len() > 1 {
            url.push_str(&format!("&s={}", codes[1..].join(",")));
        }
        let text = self.get_checked(&url).await?;
        let v: Value = serde_json::from_str(&text).map_err(|_| Fail::BadBody)?;
        if v.is_string() {
            return Err(Fail::BadBody);
        }
        let map: BTreeMap<String, String> =
            syms.iter().map(|s| (eodhd_code(s), s.clone())).collect();
        let batch = normalize_live(&v, &map, &wib_date(self.now_ms()));
        if batch.quotes.is_empty() {
            return Err(if batch.bad.is_empty() {
                Fail::BadBody
            } else {
                Fail::Ticker
            });
        }
        Ok(batch)
    }

    async fn eodhd_eod(&self, sym: &str, now: i64) -> Result<EodPoint, Fail> {
        let key = self.cfg.eodhd_key.as_ref().ok_or(Fail::NoKey)?;
        let from = wib_date(now - 7 * 86_400_000);
        let to = wib_date(now);
        let url = format!(
            "{}/api/eod/{}?api_token={}&fmt=json&from={}&to={}",
            self.cfg.eodhd_base,
            eodhd_code(sym),
            key,
            from,
            to
        );
        let text = self.get_checked(&url).await?;
        let v: Value = serde_json::from_str(&text).map_err(|_| Fail::BadBody)?;
        pick_last_eod(&v).ok_or(Fail::BadBody)
    }

    async fn fmp_quote(&self, sym: &str) -> Result<FetchedQuote, Fail> {
        let key = self.cfg.fmp_key.as_ref().ok_or(Fail::NoKey)?;
        let url = format!(
            "{}/stable/quote?symbol={}&apikey={}",
            self.cfg.fmp_base, sym, key
        );
        let text = self.get_checked(&url).await?;
        let v: Value = serde_json::from_str(&text).map_err(|_| Fail::BadBody)?;
        let obj = v
            .as_array()
            .and_then(|a| a.first())
            .and_then(Value::as_object)
            .ok_or(Fail::Ticker)?;
        let price = obj
            .get("price")
            .and_then(Value::as_f64)
            .ok_or(Fail::BadBody)?;
        let asof = obj
            .get("timestamp")
            .and_then(Value::as_i64)
            .map(|t| wib_date(if t > 10_000_000_000 { t } else { t * 1000 }))
            .unwrap_or_else(|| wib_date(self.now_ms()));
        Ok(FetchedQuote {
            symbol: sym.to_string(),
            price,
            previous_close: obj.get("previousClose").and_then(Value::as_f64),
            asof,
        })
    }
}

fn cache_quote(q: FetchedQuote, now: i64, today: &str, source: &str) -> CachedQuote {
    CachedQuote {
        symbol: q.symbol,
        price: q.price,
        previous_close: q.previous_close,
        asof: q.asof,
        fetched_at: iso_wib(now),
        source: source.into(),
        last_fetch_wib_date: today.into(),
    }
}

fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}

/// Penjadwal latar: bangun tiap 5 menit, keputusan penuh di [`decide`].
pub fn spawn_scheduler(handle: Arc<StocksHandle>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(TICK_SECS));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            let r = handle.trigger(false).await;
            if r.get("refreshed") == Some(&Value::Bool(true)) {
                println!(
                    "saham: {}",
                    r.get("reason").and_then(Value::as_str).unwrap_or("")
                );
            }
        }
    });
}

// --------------------------------------------------------------------- uji

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(date: &str, hhmm: &str) -> i64 {
        let (h, m) = hhmm.split_once(':').unwrap();
        let minute = h.parse::<u32>().unwrap() * 60 + m.parse::<u32>().unwrap();
        epoch_ms_from_wib(date, minute).unwrap()
    }

    fn gate(now: i64, manual: bool) -> GateCtx<'static> {
        GateCtx {
            now_ms: now,
            last_fetch_date: None,
            live_attempt_date: None,
            live_failed_nonquota: false,
            manual,
        }
    }

    #[test]
    fn routing_jk_and_us() {
        assert_eq!(route("BBCA"), Provider::Eodhd);
        assert_eq!(route("BBCA.JK"), Provider::Eodhd);
        assert_eq!(route("tlkm.jk"), Provider::Eodhd);
        assert_eq!(route("AAPL.US"), Provider::Eodhd);
        assert_eq!(route("NVDA.US"), Provider::Eodhd);
        assert_eq!(route("TSM.US"), Provider::Eodhd);
        assert_eq!(route("TSLA.US"), Provider::Eodhd);
        assert_eq!(route("nvda.us"), Provider::Eodhd);
        assert_eq!(route("SAP.DE"), Provider::Fmp);
        assert_eq!(eodhd_code("BBCA"), "BBCA.JK");
        assert_eq!(eodhd_code("BBCA.JK"), "BBCA.JK");
        assert_eq!(eodhd_code("NVDA.US"), "NVDA.US");
        assert_eq!(eodhd_code("TSM.US"), "TSM.US");
        assert_eq!(eodhd_code("TSLA.US"), "TSLA.US");
        assert_eq!(eodhd_code("nvda.us"), "NVDA.US");
        assert_eq!(symbol_parts("BBCA"), ("BBCA", None));
    }

    #[test]
    fn wib_clock_basics() {
        // 1970-01-01 00:00 UTC = 07:00 WIB, Kamis.
        assert_eq!(wib_date(0), "1970-01-01");
        assert_eq!(wib_hms(0), "07:00:00");
        assert_eq!(wib_weekday(0), 3);
        // 2026-11-07 = Sabtu, 2026-11-06 = Jumat.
        assert!(wib_is_weekend(ms("2026-11-07", "12:00")));
        assert!(!wib_is_weekend(ms("2026-11-06", "12:00")));
        assert_eq!(wib_minute_of_day(ms("2026-11-06", "15:01")), 15 * 60 + 1);
        assert!(iso_wib(ms("2026-11-06", "15:01")).starts_with("2026-11-06T15:01"));
    }

    #[test]
    fn gate_weekend_blocks_even_manual() {
        let sat = ms("2026-11-07", "15:30");
        assert!(matches!(decide(&gate(sat, false)), Action::Skip(_)));
        assert!(matches!(decide(&gate(sat, true)), Action::Skip(_)));
        let sun = ms("2026-11-08", "16:10");
        assert!(matches!(decide(&gate(sun, false)), Action::Skip(_)));
        let fri = ms("2026-11-06", "15:30");
        assert_eq!(decide(&gate(fri, false)), Action::Live);
    }

    #[test]
    fn gate_live_window() {
        let early = ms("2026-11-06", "14:59");
        assert!(matches!(decide(&gate(early, false)), Action::Skip(_)));
        assert_eq!(decide(&gate(early, true)), Action::Live);
        let open = ms("2026-11-06", "15:00");
        assert_eq!(decide(&gate(open, false)), Action::Live);
    }

    #[test]
    fn gate_once_per_day() {
        let now = ms("2026-11-06", "15:30");
        let mut ctx = gate(now, false);
        ctx.last_fetch_date = Some("2026-11-06");
        assert!(matches!(decide(&ctx), Action::Skip(r) if r.contains("hari ini")));
        // Tanggal lama tidak memblokir.
        let mut ok = gate(now, false);
        ok.last_fetch_date = Some("2026-11-05");
        assert_eq!(decide(&ok), Action::Live);
    }

    #[test]
    fn gate_eod_retry_after_nonquota_failure() {
        let retry = ms("2026-11-06", "16:05");
        let mut ctx = gate(retry, false);
        ctx.live_attempt_date = Some("2026-11-06");
        ctx.live_failed_nonquota = true;
        assert_eq!(decide(&ctx), Action::Eod);
        // Sebelum 16:05 tetap tunggu.
        let mut early = gate(ms("2026-11-06", "16:04"), false);
        early.live_attempt_date = Some("2026-11-06");
        early.live_failed_nonquota = true;
        assert!(matches!(decide(&early), Action::Skip(_)));
        // Gagal karena kuota: tidak ada percobaan ulang.
        let mut quota = gate(retry, false);
        quota.live_attempt_date = Some("2026-11-06");
        quota.live_failed_nonquota = false;
        assert!(matches!(decide(&quota), Action::Skip(_)));
    }

    #[test]
    fn quota_guard_decisions() {
        assert_eq!(quota_decision(0, 20, 0, 3), Quota::Proceed);
        assert_eq!(quota_decision(17, 20, 0, 3), Quota::Proceed); // sisa 3 = kebutuhan
        assert_eq!(quota_decision(19, 20, 0, 3), Quota::Limited(1));
        assert_eq!(quota_decision(20, 20, 0, 1), Quota::Defer);
        assert_eq!(quota_decision(25, 20, 0, 1), Quota::Defer);
        // Bonus sekali pakai menambah sisa.
        assert_eq!(quota_decision(20, 20, 5, 3), Quota::Proceed);
        assert_eq!(quota_decision(24, 20, 1, 3), Quota::Limited(1));
        assert_eq!(quota_decision(0, 20, 0, 0), Quota::Proceed);
    }

    #[test]
    fn live_normalization_single_object() {
        let v: Value = serde_json::from_str(
            r#"{"code":"BBCA.JK","timestamp":1762326000,"close":5025,"previousClose":5000,"change":25}"#,
        )
        .unwrap();
        let map = BTreeMap::from([("BBCA.JK".to_string(), "BBCA".to_string())]);
        let b = normalize_live(&v, &map, "2026-11-05");
        assert_eq!(b.quotes.len(), 1);
        assert_eq!(b.quotes[0].symbol, "BBCA");
        assert_eq!(b.quotes[0].price, 5025.0);
        assert_eq!(b.quotes[0].previous_close, Some(5000.0));
        assert_eq!(b.quotes[0].asof, wib_date(1762326000 * 1000));
    }

    #[test]
    fn live_normalization_batch_object() {
        let v: Value = serde_json::from_str(
            r#"{"BBCA.JK":{"code":"BBCA.JK","timestamp":1762326000,"close":5025},
                "TLKM.JK":{"code":"TLKM.JK","timestamp":1762326000,"close":3100},
                "XXXX.JK":{"code":"XXXX.JK","error":"Ticker not found"}}"#,
        )
        .unwrap();
        let map = BTreeMap::from([
            ("BBCA.JK".to_string(), "BBCA".to_string()),
            ("TLKM.JK".to_string(), "TLKM".to_string()),
        ]);
        let b = normalize_live(&v, &map, "2026-11-05");
        let mut syms: Vec<_> = b.quotes.iter().map(|q| q.symbol.as_str()).collect();
        syms.sort();
        assert_eq!(syms, vec!["BBCA", "TLKM"]);
        assert_eq!(b.bad, vec!["XXXX.JK".to_string()]);
        assert_eq!(
            normalize_live(&Value::String("err".into()), &map, "2026-11-05")
                .quotes
                .len(),
            0
        );
    }

    #[test]
    fn eod_takes_last_element() {
        let v: Value = serde_json::from_str(
            r#"[{"date":"2026-11-03","close":4100},{"date":"2026-11-04","close":4120},{"date":"2026-11-05","close":4150}]"#,
        )
        .unwrap();
        let p = pick_last_eod(&v).unwrap();
        assert_eq!(p.date, "2026-11-05");
        assert_eq!(p.price, 4150.0);
        assert!(pick_last_eod(&serde_json::json!([])).is_none());
        assert!(pick_last_eod(&serde_json::json!({"error":"x"})).is_none());
    }

    #[test]
    fn selection_under_cap_uses_all() {
        let syms: Vec<(String, bool)> = (0..3).map(|i| (format!("S{i}"), false)).collect();
        assert_eq!(select_daily(&syms, 0).len(), 3);
        let twenty: Vec<(String, bool)> = (0..20).map(|i| (format!("S{i}"), false)).collect();
        assert_eq!(select_daily(&twenty, 0), select_daily(&twenty, 9));
    }

    #[test]
    fn selection_over_cap_rotates_round_robin() {
        let syms: Vec<(String, bool)> = (0..45).map(|i| (format!("S{i}"), false)).collect();
        let d0 = select_daily(&syms, 0);
        let d1 = select_daily(&syms, 1);
        let d2 = select_daily(&syms, 2);
        assert_eq!(d0.len(), DAILY_BATCH_CAP);
        assert_eq!(d0[0], "S0"); // hari pertama: 20 pertama urutan file
        assert_ne!(d0, d1);
        assert_ne!(d1, d2);
        // Tiga hari menutup semua 45 simbol.
        let mut all: Vec<String> = d0.into_iter().chain(d1).chain(d2).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 45);
    }

    #[test]
    fn selection_priority_first() {
        let mut syms: Vec<(String, bool)> = (0..30).map(|i| (format!("S{i}"), false)).collect();
        syms[25].1 = true;
        let set = select_daily(&syms, 0);
        assert_eq!(set.len(), 20);
        assert_eq!(set[0], "S25");
        assert!(set.iter().any(|s| s == "S0")); // sisanya bergilir
                                                // Prioritas lebih dari 20 dipotong.
        let all_prio: Vec<(String, bool)> = (0..25).map(|i| (format!("S{i}"), true)).collect();
        let set2 = select_daily(&all_prio, 0);
        assert_eq!(set2.len(), 20);
        assert!(!set2.iter().any(|s| s == "S20"));
    }

    #[test]
    fn cache_roundtrip_and_stale_flag() {
        let dir = std::env::temp_dir().join(format!("hq-stocks-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = dir.to_str().unwrap();
        assert!(load_cache(data).symbols.is_empty());
        let mut cache = StocksCache::default();
        cache.symbols.insert(
            "BBCA".into(),
            CachedQuote {
                symbol: "BBCA".into(),
                price: 5025.0,
                previous_close: None,
                asof: "2026-11-06".into(),
                fetched_at: "2026-11-06T15:01:00+07:00".into(),
                source: "EODHD-Live".into(),
                last_fetch_wib_date: "2026-11-06".into(),
            },
        );
        cache.quotes_updated_at = Some("2026-11-06T15:01:00+07:00".into());
        save_cache(data, &cache).unwrap();
        let back = load_cache(data);
        assert_eq!(back.symbols["BBCA"].price, 5025.0);
        assert_eq!(back.quotes_updated_at, cache.quotes_updated_at);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn iso_wib_roundtrip() {
        let t = ms("2026-11-06", "16:05");
        assert_eq!(iso_wib(t), "2026-11-06T16:05:00+07:00");
        assert_eq!(wib_minute_of_day(t), 16 * 60 + 5);
    }
}
