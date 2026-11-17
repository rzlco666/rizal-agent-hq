//! Performa portofolio saham: posisi manual + hitung dari cache harga.
//!
//! Aturan (SA HQ-PORTFOLIO-SA):
//! - Posisi hidup di `data/portfolio.json` (file user, tidak pernah di-commit).
//! - Cost basis = `avg_buy_price` input manual; TIDAK PERNAH ditebak dari
//!   `stocks.json:price` (artinya ambigu: harga beli / catatan / penutupan).
//! - Semua harga dari cache (`load_cache`); modul ini tidak memanggil provider,
//!   tidak memakai kuota, tidak memegang kunci API.
//! - Data yang tidak ada = null ("—" di UI), BUKAN 0.
//! - Tidak ada nasihat investasi: tidak ada rekomendasi, prediksi, auto-trading.

use hq_core::redact::redact;
use hq_core::timefmt::wib_date;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::stocks_price::{
    load_cache, read_stocks, wib_minute_of_day, CachedQuote, Stock, StocksCache, StocksFile,
};

// ------------------------------------------------------------------ model

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Holding {
    pub symbol: String,
    pub shares: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_buy_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bought_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PortfolioFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usd_idr_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_asof: Option<String>,
    #[serde(default)]
    pub holdings: Vec<Holding>,
}

pub enum PortfolioRead {
    Missing,
    Invalid(String),
    Ok(PortfolioFile),
}

pub fn portfolio_path(data_dir: &str) -> String {
    format!("{data_dir}/portfolio.json")
}

pub fn read_portfolio(path: &str) -> PortfolioRead {
    match std::fs::read_to_string(path) {
        Ok(t) => match serde_json::from_str::<PortfolioFile>(&t) {
            Ok(v) => PortfolioRead::Ok(v),
            Err(e) => PortfolioRead::Invalid(format!("portfolio.json tidak bisa dibaca: {e}")),
        },
        Err(_) => PortfolioRead::Missing,
    }
}

/// Tulis atomik (tmp + rename) — pola `save_cache` modul harga.
pub fn save_portfolio(path: &str, pf: &PortfolioFile) -> Result<(), String> {
    let tmp = format!("{path}.tmp");
    let body = serde_json::to_string_pretty(pf).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Normalisasi simbol: "BBCA" ≡ "BBCA.JK"; "AAPL.US" tetap; huruf besar semua.
pub fn norm_symbol(sym: &str) -> String {
    let s = sym.trim().to_ascii_uppercase();
    match s.rsplit_once('.') {
        Some((base, suf)) if !base.is_empty() && !suf.is_empty() => {
            if suf == "JK" {
                format!("{base}.JK")
            } else {
                s
            }
        }
        _ => format!("{s}.JK"),
    }
}

/// Tanggal YYYY-MM-DD valid (kalender nyata) atau bukan.
pub fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !s
        .bytes()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let y: i64 = s[0..4].parse().unwrap_or(0);
    let m: u32 = s[5..7].parse().unwrap_or(0);
    let d: u32 = s[8..10].parse().unwrap_or(0);
    if !(1..=12).contains(&m) || d == 0 {
        return false;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let dim = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ][(m - 1) as usize];
    d <= dim
}

// -------------------------------------------------------------- validasi

/// Input mentah POST/PATCH holding (lot ATAU lembar — bukan keduanya).
#[derive(Debug, Default, Deserialize)]
pub struct HoldingInput {
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub shares: Option<f64>,
    #[serde(default)]
    pub lots: Option<f64>,
    #[serde(default)]
    pub avg_buy_price: Option<f64>,
    #[serde(default)]
    pub bought_at: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Validasi input holding terhadap daftar pantauan (`stocks.json`).
/// `today` = tanggal WIB hari ini ("YYYY-MM-DD") untuk batas `bought_at`.
/// Mengembalikan holding yang sudah dinormalisasi (simbol = ejaan di stocks.json).
pub fn validate_holding(
    input: &HoldingInput,
    stocks: &[Stock],
    today: &str,
) -> Result<Holding, String> {
    let raw = input.symbol.as_deref().unwrap_or("").trim();
    if raw.is_empty() {
        return Err("Simbol wajib diisi.".into());
    }
    let norm = norm_symbol(raw);
    let stock = stocks
        .iter()
        .find(|s| norm_symbol(&s.symbol) == norm)
        .ok_or_else(|| {
            format!("Simbol {raw} belum ada di stocks.json — tambahkan dulu di panel Saham.")
        })?;
    let shares = match (input.shares, input.lots) {
        (Some(sh), None) => sh,
        (None, Some(lots)) => lots * 100.0,
        (Some(_), Some(_)) => {
            return Err("Isi lot ATAU lembar, bukan keduanya.".into());
        }
        (None, None) => return Err("Isi jumlah lembar atau lot (harus > 0).".into()),
    };
    if shares <= 0.0 || !shares.is_finite() {
        return Err("Jumlah lembar/lot harus lebih dari 0.".into());
    }
    if let Some(p) = input.avg_buy_price {
        if p <= 0.0 || !p.is_finite() {
            return Err("Harga beli rata-rata harus lebih dari 0 bila diisi.".into());
        }
    }
    if let Some(d) = input.bought_at.as_deref() {
        let d = d.trim();
        if !valid_date(d) {
            return Err("Tanggal beli harus format YYYY-MM-DD yang valid.".into());
        }
        if d > today {
            return Err("Tanggal beli tidak boleh di masa depan.".into());
        }
    }
    if let Some(n) = input.note.as_deref() {
        if n.chars().count() > 200 {
            return Err("Catatan maksimal 200 karakter.".into());
        }
    }
    Ok(Holding {
        symbol: stock.symbol.clone(),
        shares,
        avg_buy_price: input.avg_buy_price,
        bought_at: input.bought_at.as_deref().map(|d| d.trim().to_string()),
        note: input.note.as_deref().map(|n| redact(n.trim())),
    })
}

// ------------------------------------------------------------------ hitung

#[derive(Debug, Clone, Serialize)]
pub struct HoldingView {
    pub symbol: String,
    pub name: String,
    pub shares: f64,
    pub lots: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_buy_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bought_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub currency: String,
    pub live: Option<Value>,
    pub value: Option<f64>,
    pub cost: Option<f64>,
    pub pnl: Option<f64>,
    pub pnl_pct: Option<f64>,
    pub day_pnl: Option<f64>,
    pub day_pnl_pct: Option<f64>,
    pub weight_pct: Option<f64>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrencyTotal {
    pub currency: String,
    pub value: Option<f64>,
    pub cost: Option<f64>,
    pub pnl: Option<f64>,
    pub pnl_pct: Option<f64>,
    pub day_pnl: Option<f64>,
    pub day_pnl_pct: Option<f64>,
    pub holdings: usize,
    pub priced: usize,
    pub unpriced: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TotalsView {
    pub per_currency: Vec<CurrencyTotal>,
    pub grand_idr: Option<f64>,
    pub fx: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioView {
    pub holdings: Vec<HoldingView>,
    pub totals: TotalsView,
    pub prices_asof: Option<String>,
    pub notes: Vec<String>,
}

struct CurrencyAcc {
    holdings: usize,
    priced: usize,
    value_sum: f64,
    cost_sum: f64,
    cost_known: bool,
    pnl_sum: f64,
    pnl_known: bool,
    day_sum: f64,
    day_known: bool,
    prev_sum: f64,
}

/// Mesin hitung murni: tanpa IO, tanpa network. Semua harga dari `cache`.
/// `today` = tanggal WIB hari ini ("YYYY-MM-DD") untuk label `stale`.
pub fn compute(
    pf: &PortfolioFile,
    stocks: &[Stock],
    cache: &StocksCache,
    today: &str,
) -> PortfolioView {
    let stocks_by_norm: HashMap<String, &Stock> =
        stocks.iter().map(|s| (norm_symbol(&s.symbol), s)).collect();
    let cache_by_norm: HashMap<String, &CachedQuote> = cache
        .symbols
        .iter()
        .map(|(k, q)| (norm_symbol(k), q))
        .collect();

    let mut views: Vec<HoldingView> = Vec::new();
    let mut notes: Vec<String> = vec!["belum termasuk dividen".to_string()];
    let mut unpriced = 0usize;

    for h in &pf.holdings {
        let norm = norm_symbol(&h.symbol);
        let stock = stocks_by_norm.get(&norm);
        let currency = stock
            .map(|s| s.currency.clone())
            .unwrap_or_else(|| "IDR".into());
        let name = stock.map(|s| s.name.clone()).unwrap_or_default();
        let quote = cache_by_norm.get(&norm).copied();
        let mut reasons: Vec<String> = vec![];
        let live = quote.map(|q| {
            json!({
                "price": q.price,
                "previous_close": q.previous_close,
                "asof": q.asof,
                "fetched_at": q.fetched_at,
                "source": q.source,
                "stale": q.last_fetch_wib_date != today,
            })
        });
        let value = quote.map(|q| h.shares * q.price);
        let cost = h.avg_buy_price.map(|p| h.shares * p);
        let pnl = match (value, cost) {
            (Some(v), Some(c)) => Some(v - c),
            _ => None,
        };
        let pnl_pct = match (pnl, cost) {
            (Some(p), Some(c)) if c > 0.0 => Some(p / c * 100.0),
            _ => None,
        };
        let day_pnl = quote.and_then(|q| q.previous_close.map(|pc| (q.price - pc) * h.shares));
        let day_pnl_pct = quote.and_then(|q| {
            q.previous_close.and_then(|pc| {
                if pc > 0.0 {
                    Some((q.price - pc) / pc * 100.0)
                } else {
                    None
                }
            })
        });
        if quote.is_none() {
            reasons.push("harga belum tersedia".into());
            unpriced += 1;
        }
        if h.avg_buy_price.is_none() {
            reasons.push("harga beli belum diisi".into());
        }
        if let Some(q) = quote {
            if q.previous_close.is_none() {
                reasons.push("tanpa previous close (EOD)".into());
            }
        }
        views.push(HoldingView {
            symbol: h.symbol.clone(),
            name,
            shares: h.shares,
            lots: h.shares / 100.0,
            avg_buy_price: h.avg_buy_price,
            bought_at: h.bought_at.clone(),
            note: h.note.clone(),
            currency,
            live,
            value,
            cost,
            pnl,
            pnl_pct,
            day_pnl,
            day_pnl_pct,
            weight_pct: None,
            reasons,
        });
    }

    // Total per mata uang (urutan kemunculan pertama dipertahankan).
    let mut order: Vec<String> = vec![];
    let mut accs: HashMap<String, CurrencyAcc> = HashMap::new();
    for v in &views {
        let acc = accs.entry(v.currency.clone()).or_insert_with(|| {
            order.push(v.currency.clone());
            CurrencyAcc {
                holdings: 0,
                priced: 0,
                value_sum: 0.0,
                cost_sum: 0.0,
                cost_known: true,
                pnl_sum: 0.0,
                pnl_known: true,
                day_sum: 0.0,
                day_known: true,
                prev_sum: 0.0,
            }
        });
        acc.holdings += 1;
        if let Some(val) = v.value {
            acc.priced += 1;
            acc.value_sum += val;
        }
        match v.cost {
            Some(c) => acc.cost_sum += c,
            None => acc.cost_known = false,
        }
        match v.pnl {
            Some(p) => acc.pnl_sum += p,
            None => acc.pnl_known = false,
        }
        match v.day_pnl {
            Some(d) => acc.day_sum += d,
            None => acc.day_known = false,
        }
        // Σ previous_close × shares (penyebut % harian total).
        if let Some(live) = &v.live {
            if let Some(pc) = live.get("previous_close").and_then(Value::as_f64) {
                acc.prev_sum += pc * v.shares;
            }
        }
    }
    let mut totals: Vec<CurrencyTotal> = vec![];
    for cur in &order {
        let a = &accs[cur];
        let value = if a.priced > 0 {
            Some(a.value_sum)
        } else {
            None
        };
        let cost = if a.cost_known { Some(a.cost_sum) } else { None };
        let pnl = if a.pnl_known { Some(a.pnl_sum) } else { None };
        let pnl_pct = match (pnl, cost) {
            (Some(p), Some(c)) if c > 0.0 => Some(p / c * 100.0),
            _ => None,
        };
        let day_pnl = if a.day_known { Some(a.day_sum) } else { None };
        let day_pnl_pct = match (day_pnl, a.prev_sum) {
            (Some(d), prev) if prev > 0.0 => Some(d / prev * 100.0),
            _ => None,
        };
        totals.push(CurrencyTotal {
            currency: cur.clone(),
            value,
            cost,
            pnl,
            pnl_pct,
            day_pnl,
            day_pnl_pct,
            holdings: a.holdings,
            priced: a.priced,
            unpriced: a.holdings - a.priced,
        });
    }

    // Bobot tiap holding terhadap total mata uangnya.
    for v in views.iter_mut() {
        if let (Some(val), Some(tot)) = (
            v.value,
            totals
                .iter()
                .find(|t| t.currency == v.currency)
                .and_then(|t| t.value),
        ) {
            if tot > 0.0 {
                v.weight_pct = Some(val / tot * 100.0);
            }
        }
    }

    // Grand total IDR: subtotal IDR + subtotal USD × kurs manual.
    // v1 hanya mengonversi USD; mata uang lain tampil sebagai subtotal sendiri.
    let has_non_idr = totals.iter().any(|t| t.currency != "IDR");
    let fx = match (pf.usd_idr_rate, pf.rate_asof.as_deref()) {
        (Some(rate), Some(asof)) => Some(json!({ "rate": rate, "asof": asof })),
        _ => None,
    };
    let mut grand_idr: Option<f64> = None;
    if !has_non_idr {
        grand_idr = totals
            .iter()
            .find(|t| t.currency == "IDR")
            .and_then(|t| t.value);
    } else if let Some(rate) = pf.usd_idr_rate {
        let idr = totals
            .iter()
            .find(|t| t.currency == "IDR")
            .and_then(|t| t.value)
            .unwrap_or(0.0);
        let usd = totals
            .iter()
            .find(|t| t.currency == "USD")
            .and_then(|t| t.value);
        if let Some(u) = usd {
            grand_idr = Some(idr + u * rate);
        } else if totals
            .iter()
            .all(|t| t.currency == "IDR" || t.value.is_none())
        {
            grand_idr = Some(idr);
        }
    }
    if unpriced > 0 {
        notes.push(format!(
            "{unpriced} posisi belum ada harga — tidak masuk total"
        ));
    }
    if has_non_idr && grand_idr.is_none() {
        notes.push("Grand total IDR tidak tersedia — isi kurs USD→IDR".into());
    }
    if totals
        .iter()
        .any(|t| t.currency != "IDR" && t.currency != "USD")
    {
        notes.push(
            "Mata uang selain IDR/USD tidak masuk grand total (v1 hanya konversi USD)".into(),
        );
    }

    PortfolioView {
        holdings: views,
        totals: TotalsView {
            per_currency: totals,
            grand_idr,
            fx,
        },
        prices_asof: cache.quotes_updated_at.clone(),
        notes,
    }
}

// ---------------------------------------------------------------- snapshot

pub fn ensure_portfolio_schema(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS portfolio_snapshots(
            date TEXT NOT NULL,
            currency TEXT NOT NULL,
            total_value REAL NOT NULL,
            total_cost REAL,
            total_pnl REAL,
            day_pnl REAL,
            holdings_count INTEGER NOT NULL,
            priced_count INTEGER NOT NULL,
            stale INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (date, currency)
        );",
    )
    .expect("skema portfolio_snapshots");
}

/// Ambil snapshot harian (idempotent, upsert per (tanggal, mata uang)).
/// `now` = jam efektif (jam palsu uji ikut lewat pemanggil).
/// Hanya bila ada fetch segar hari itu (`last_fetch_wib_date == hari ini`);
/// akhir pekan / hari tanpa fetch → TIDAK ADA baris baru.
pub fn take_snapshot(conn: &rusqlite::Connection, data_dir: &str, now: i64) -> Value {
    let today = wib_date(now);
    let cache = load_cache(data_dir);
    if cache.last_fetch_wib_date.as_deref() != Some(today.as_str()) {
        return json!({ "snapshotted": false, "reason": "tidak ada fetch segar hari ini" });
    }
    let pf = match read_portfolio(&portfolio_path(data_dir)) {
        PortfolioRead::Ok(v) => v,
        PortfolioRead::Missing => {
            return json!({ "snapshotted": false, "reason": "belum ada posisi" });
        }
        PortfolioRead::Invalid(e) => return json!({ "snapshotted": false, "reason": e }),
    };
    if pf.holdings.is_empty() {
        return json!({ "snapshotted": false, "reason": "belum ada posisi" });
    }
    let stocks = match read_stocks(&format!("{data_dir}/stocks.json")) {
        StocksFile::Ok(v) => v,
        _ => vec![],
    };
    let view = compute(&pf, &stocks, &cache, &today);
    let mut currencies: Vec<String> = vec![];
    for t in &view.totals.per_currency {
        let stale = view
            .holdings
            .iter()
            .filter(|v| v.currency == t.currency)
            .any(|v| {
                v.live
                    .as_ref()
                    .and_then(|l| l.get("asof"))
                    .and_then(Value::as_str)
                    .is_some_and(|asof| asof < today.as_str())
            });
        let r = conn.execute(
            "INSERT INTO portfolio_snapshots(date, currency, total_value, total_cost, total_pnl, day_pnl, holdings_count, priced_count, stale, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
             ON CONFLICT(date, currency) DO UPDATE SET total_value=excluded.total_value, total_cost=excluded.total_cost, total_pnl=excluded.total_pnl, day_pnl=excluded.day_pnl, holdings_count=excluded.holdings_count, priced_count=excluded.priced_count, stale=excluded.stale, created_at=excluded.created_at",
            rusqlite::params![
                today,
                t.currency,
                t.value.unwrap_or(0.0),
                t.cost,
                t.pnl,
                t.day_pnl,
                t.holdings as i64,
                t.priced as i64,
                stale as i64,
                now
            ],
        );
        if let Err(e) = r {
            return json!({ "snapshotted": false, "reason": format!("gagal simpan snapshot: {e}") });
        }
        currencies.push(t.currency.clone());
    }
    json!({ "snapshotted": true, "date": today, "currencies": currencies })
}

/// Riwayat snapshot: baris `portfolio_snapshots` sejak `days` hari lalu.
/// `currency` kosong = semua mata uang.
pub fn read_history(
    conn: &rusqlite::Connection,
    days: i64,
    currency: Option<&str>,
    now: i64,
) -> Vec<Value> {
    let since = wib_date(now - days * 86_400_000);
    let mut rows = vec![];
    let (sql, params): (&str, Vec<String>) = match currency {
        Some(c) => (
            "SELECT date, currency, total_value, total_cost, total_pnl, day_pnl, holdings_count, priced_count, stale FROM portfolio_snapshots WHERE date >= ?1 AND currency = ?2 ORDER BY date, currency",
            vec![since, c.to_string()],
        ),
        None => (
            "SELECT date, currency, total_value, total_cost, total_pnl, day_pnl, holdings_count, priced_count, stale FROM portfolio_snapshots WHERE date >= ?1 ORDER BY date, currency",
            vec![since],
        ),
    };
    if let Ok(mut stmt) = conn.prepare(sql) {
        let iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |r| {
            Ok(json!({
                "date": r.get::<_, String>(0)?,
                "currency": r.get::<_, String>(1)?,
                "total_value": r.get::<_, f64>(2)?,
                "total_cost": r.get::<_, Option<f64>>(3)?,
                "total_pnl": r.get::<_, Option<f64>>(4)?,
                "day_pnl": r.get::<_, Option<f64>>(5)?,
                "holdings": r.get::<_, i64>(6)?,
                "priced": r.get::<_, i64>(7)?,
                "stale": r.get::<_, i64>(8)?,
            }))
        });
        if let Ok(iter) = iter {
            rows = iter.flatten().collect();
        }
    }
    rows
}

// -------------------------------------------------------------- penjadwal

/// Penjadwal snapshot 16:30 WIB: tick tiap 60 dtk; jalan sekali sehari setelah
/// 16:30 bila belum ada snapshot hari itu (cek DB, bukan state memori — aman
/// dari restart). Tidak ada fetch segar hari itu → `take_snapshot` menolak
/// dengan diam (tidak ada baris baru).
pub fn spawn_portfolio_scheduler(st: Arc<crate::AppState>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tick.tick().await;
            let now = st.stocks.now_ms();
            if wib_minute_of_day(now) < 16 * 60 + 30 {
                continue;
            }
            let today = wib_date(now);
            let done = {
                let Ok(db) = st.db.lock() else { continue };
                let Ok(mut stmt) =
                    db.prepare("SELECT COUNT(*) FROM portfolio_snapshots WHERE date = ?1")
                else {
                    continue;
                };
                matches!(
                    stmt.query_row(rusqlite::params![today], |r| r.get::<_, i64>(0)),
                    Ok(n) if n > 0
                )
            };
            if done {
                continue;
            }
            let out = {
                let Ok(db) = st.db.lock() else { continue };
                take_snapshot(&db, &st.data_dir, now)
            };
            if out.get("snapshotted") == Some(&Value::Bool(true)) {
                println!(
                    "portofolio: snapshot {}",
                    out.get("date").and_then(Value::as_str).unwrap_or("")
                );
            }
        }
    });
}

// --------------------------------------------------------------------- uji

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_stocks() -> Vec<Stock> {
        vec![
            Stock {
                symbol: "BBCA".into(),
                name: "Bank Central Asia".into(),
                shares: 100.0,
                price: 9000.0,
                currency: "IDR".into(),
                priority: None,
            },
            Stock {
                symbol: "AAPL.US".into(),
                name: "Apple".into(),
                shares: 2.0,
                price: 180.0,
                currency: "USD".into(),
                priority: None,
            },
            Stock {
                symbol: "TLKM".into(),
                name: "Telkom Indonesia".into(),
                shares: 500.0,
                price: 3100.0,
                currency: "IDR".into(),
                priority: None,
            },
        ]
    }

    fn fixture_cache() -> StocksCache {
        let mut cache = StocksCache {
            quotes_updated_at: Some("2026-11-06T15:01:00+07:00".into()),
            last_fetch_wib_date: Some("2026-11-06".into()),
            ..Default::default()
        };
        cache.symbols.insert(
            "BBCA".into(),
            CachedQuote {
                symbol: "BBCA".into(),
                price: 9500.0,
                previous_close: Some(9400.0),
                asof: "2026-11-06".into(),
                fetched_at: "2026-11-06T15:01:00+07:00".into(),
                source: "EODHD-Live".into(),
                last_fetch_wib_date: "2026-11-06".into(),
            },
        );
        cache.symbols.insert(
            "AAPL.US".into(),
            CachedQuote {
                symbol: "AAPL.US".into(),
                price: 180.0,
                previous_close: Some(178.0),
                asof: "2026-11-06".into(),
                fetched_at: "2026-11-06T15:01:00+07:00".into(),
                source: "FMP".into(),
                last_fetch_wib_date: "2026-11-06".into(),
            },
        );
        cache
    }

    fn fixture_portfolio() -> PortfolioFile {
        PortfolioFile {
            usd_idr_rate: None,
            rate_asof: None,
            holdings: vec![
                Holding {
                    symbol: "BBCA".into(),
                    shares: 1000.0,
                    avg_buy_price: Some(9200.0),
                    bought_at: Some("2026-03-14".into()),
                    note: Some("lot 10".into()),
                },
                Holding {
                    symbol: "AAPL.US".into(),
                    shares: 5.0,
                    avg_buy_price: Some(175.5),
                    bought_at: Some("2025-11-02".into()),
                    note: None,
                },
                Holding {
                    symbol: "TLKM".into(),
                    shares: 500.0,
                    avg_buy_price: None,
                    bought_at: None,
                    note: None,
                },
            ],
        }
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|v| (v - b).abs() < 0.01)
    }

    #[test]
    fn norm_symbol_equivalence() {
        assert_eq!(norm_symbol("BBCA"), "BBCA.JK");
        assert_eq!(norm_symbol("BBCA.JK"), "BBCA.JK");
        assert_eq!(norm_symbol("bbca.jk"), "BBCA.JK");
        assert_eq!(norm_symbol("AAPL.US"), "AAPL.US");
        assert_eq!(norm_symbol(" aapl.us "), "AAPL.US");
    }

    #[test]
    fn valid_date_calendar() {
        assert!(valid_date("2026-11-06"));
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2026-02-29"));
        assert!(!valid_date("2026-13-01"));
        assert!(!valid_date("2026-11-31"));
        assert!(!valid_date("06-11-2026"));
        assert!(!valid_date("2026-11-6"));
    }

    #[test]
    fn compute_a1_fixture_numbers() {
        let view = compute(
            &fixture_portfolio(),
            &fixture_stocks(),
            &fixture_cache(),
            "2026-11-06",
        );
        let bbca = view
            .holdings
            .iter()
            .find(|h| h.symbol == "BBCA")
            .expect("BBCA ada");
        assert!(approx(bbca.value, 9_500_000.0));
        assert!(approx(bbca.cost, 9_200_000.0));
        assert!(approx(bbca.pnl, 300_000.0));
        assert!(approx(bbca.pnl_pct, 3.260869565));
        assert!(approx(bbca.day_pnl, 100_000.0));
        assert!(approx(bbca.day_pnl_pct, 1.063829787));
        assert!(approx(bbca.weight_pct, 100.0));
        let aapl = view
            .holdings
            .iter()
            .find(|h| h.symbol == "AAPL.US")
            .expect("AAPL.US ada");
        assert!(approx(aapl.value, 900.0));
        assert!(approx(aapl.cost, 877.5));
        assert!(approx(aapl.pnl, 22.5));
        assert!(approx(aapl.pnl_pct, 2.564102564));
        assert!(approx(aapl.day_pnl, 10.0));
        assert!(approx(aapl.day_pnl_pct, 1.123595506));
        // Total IDR: harga TLKM belum ada + tanpa avg_buy_price → cost/pnl/harian null.
        let idr = view
            .totals
            .per_currency
            .iter()
            .find(|t| t.currency == "IDR")
            .expect("total IDR ada");
        assert!(approx(idr.value, 9_500_000.0));
        assert!(idr.cost.is_none());
        assert!(idr.pnl.is_none());
        assert!(idr.day_pnl.is_none());
        assert_eq!(idr.holdings, 2);
        assert_eq!(idr.priced, 1);
        // Total USD lengkap.
        let usd = view
            .totals
            .per_currency
            .iter()
            .find(|t| t.currency == "USD")
            .expect("total USD ada");
        assert!(approx(usd.value, 900.0));
        assert!(approx(usd.cost, 877.5));
        assert!(approx(usd.pnl, 22.5));
        assert!(approx(usd.day_pnl, 10.0));
        // Tanpa kurs manual → tidak ada grand total IDR.
        assert!(view.totals.grand_idr.is_none());
        assert!(view
            .notes
            .iter()
            .any(|n| n.contains("belum termasuk dividen")));
        assert!(view.notes.iter().any(|n| n.contains("belum ada harga")));
    }

    #[test]
    fn compute_a2_missing_data_is_null_not_zero() {
        let view = compute(
            &fixture_portfolio(),
            &fixture_stocks(),
            &fixture_cache(),
            "2026-11-06",
        );
        let tlkm = view
            .holdings
            .iter()
            .find(|h| h.symbol == "TLKM")
            .expect("TLKM ada");
        // Tanpa cache: nilai/P&L/harian null — BUKAN 0.
        assert!(tlkm.value.is_none());
        assert!(tlkm.pnl.is_none());
        assert!(tlkm.day_pnl.is_none());
        assert!(tlkm.live.is_none());
        assert!(tlkm
            .reasons
            .iter()
            .any(|r| r.contains("harga belum tersedia")));
        assert!(tlkm
            .reasons
            .iter()
            .any(|r| r.contains("harga beli belum diisi")));
    }

    #[test]
    fn compute_eod_without_previous_close() {
        let mut cache = fixture_cache();
        if let Some(q) = cache.symbols.get_mut("BBCA") {
            q.previous_close = None;
            q.source = "EODHD-EOD".into();
        }
        let view = compute(
            &fixture_portfolio(),
            &fixture_stocks(),
            &cache,
            "2026-11-06",
        );
        let bbca = view.holdings.iter().find(|h| h.symbol == "BBCA").unwrap();
        // Nilai + P&L tetap ada; harian null (bukan 0).
        assert!(approx(bbca.value, 9_500_000.0));
        assert!(approx(bbca.pnl, 300_000.0));
        assert!(bbca.day_pnl.is_none());
        assert!(bbca.day_pnl_pct.is_none());
        assert!(bbca.reasons.iter().any(|r| r.contains("previous close")));
    }

    #[test]
    fn compute_grand_total_with_manual_fx() {
        let mut pf = fixture_portfolio();
        pf.usd_idr_rate = Some(16_000.0);
        pf.rate_asof = Some("2026-11-06".into());
        let view = compute(&pf, &fixture_stocks(), &fixture_cache(), "2026-11-06");
        // 9.500.000 + 900 × 16.000 = 23.900.000.
        assert!(approx(view.totals.grand_idr, 23_900_000.0));
        assert_eq!(
            view.totals.fx.as_ref().unwrap().get("rate").unwrap(),
            &json!(16_000.0)
        );
    }

    #[test]
    fn validate_holding_rules() {
        let stocks = fixture_stocks();
        let today = "2026-11-06";
        // Lot → lembar.
        let ok = validate_holding(
            &HoldingInput {
                symbol: Some("BBCA".into()),
                shares: None,
                lots: Some(10.0),
                avg_buy_price: Some(9200.0),
                bought_at: Some("2026-03-14".into()),
                note: None,
            },
            &stocks,
            today,
        )
        .expect("lot 10 valid");
        assert_eq!(ok.shares, 1000.0);
        assert_eq!(ok.symbol, "BBCA");
        // Simbol di luar stocks.json ditolak.
        assert!(validate_holding(
            &HoldingInput {
                symbol: Some("GOTO".into()),
                shares: Some(100.0),
                ..Default::default()
            },
            &stocks,
            today,
        )
        .is_err());
        // Lot DAN lembar bersamaan ditolak.
        assert!(validate_holding(
            &HoldingInput {
                symbol: Some("BBCA".into()),
                shares: Some(100.0),
                lots: Some(1.0),
                ..Default::default()
            },
            &stocks,
            today,
        )
        .is_err());
        // Lembar 0 / negatif ditolak.
        assert!(validate_holding(
            &HoldingInput {
                symbol: Some("BBCA".into()),
                shares: Some(0.0),
                ..Default::default()
            },
            &stocks,
            today,
        )
        .is_err());
        // Harga beli 0 ditolak.
        assert!(validate_holding(
            &HoldingInput {
                symbol: Some("BBCA".into()),
                shares: Some(100.0),
                avg_buy_price: Some(0.0),
                ..Default::default()
            },
            &stocks,
            today,
        )
        .is_err());
        // Tanggal beli di masa depan ditolak.
        assert!(validate_holding(
            &HoldingInput {
                symbol: Some("BBCA".into()),
                shares: Some(100.0),
                bought_at: Some("2026-11-07".into()),
                ..Default::default()
            },
            &stocks,
            today,
        )
        .is_err());
        // Catatan > 200 karakter ditolak.
        assert!(validate_holding(
            &HoldingInput {
                symbol: Some("BBCA".into()),
                shares: Some(100.0),
                note: Some("x".repeat(201)),
                ..Default::default()
            },
            &stocks,
            today,
        )
        .is_err());
        // "BBCA.JK" diterima dan dinormalisasi ke ejaan stocks.json.
        let ok2 = validate_holding(
            &HoldingInput {
                symbol: Some("BBCA.JK".into()),
                shares: Some(50.0),
                ..Default::default()
            },
            &stocks,
            today,
        )
        .expect("BBCA.JK setara BBCA");
        assert_eq!(ok2.symbol, "BBCA");
    }
}
