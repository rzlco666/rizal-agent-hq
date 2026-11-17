//! Uji end-to-end performa portofolio saham: binary asli + fixture file.
//!
//! Tidak pernah memakai kunci asli (tidak ada EODHD_API_KEY/FMP_API_KEY di env)
//! dan tidak pernah memanggil provider — semua harga dari fixture cache.
//! Server dijalankan dengan data dir & port sementara; layanan live tidak disentuh.
//! "Hari ini" dipalsukan ke Jumat 2026-11-06 16:00 WIB (HQ_STOCKS_FAKE_WIB_DATE)
//! supaya snapshot diizinkan (ada fetch segar hari itu) dan scheduler 16:30
//! tidak menembak selama uji berjalan.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const FAKE_DATE: &str = "2026-11-06";

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn write_stocks(dir: &Path) {
    let body = r#"[
  {"symbol": "BBCA", "name": "Bank Central Asia", "shares": 100, "price": 9000, "currency": "IDR"},
  {"symbol": "TLKM", "name": "Telkom Indonesia", "shares": 500, "price": 3100, "currency": "IDR"},
  {"symbol": "AAPL.US", "name": "Apple", "shares": 2, "price": 180, "currency": "USD"}
]"#;
    std::fs::write(dir.join("stocks.json"), body).unwrap();
}

/// Cache fixture: BBCA + AAPL.US sudah ter-fetch hari ini; TLKM belum pernah.
fn write_cache(dir: &Path, last_fetch: &str) {
    let body = format!(
        r#"{{
  "quotes_updated_at": "2026-11-06T15:01:00+07:00",
  "last_fetch_wib_date": "{last_fetch}",
  "symbols": {{
    "BBCA": {{"symbol": "BBCA", "price": 9500.0, "previous_close": 9400.0, "asof": "2026-11-06", "fetched_at": "2026-11-06T15:01:00+07:00", "source": "EODHD-Live", "last_fetch_wib_date": "{last_fetch}"}},
    "AAPL.US": {{"symbol": "AAPL.US", "price": 180.0, "previous_close": 178.0, "asof": "2026-11-06", "fetched_at": "2026-11-06T15:01:00+07:00", "source": "FMP", "last_fetch_wib_date": "{last_fetch}"}}
  }}
}}"#
    );
    std::fs::write(dir.join("stocks_cache.json"), body).unwrap();
}

fn write_portfolio(dir: &Path) {
    let body = r#"{
  "holdings": [
    {"symbol": "BBCA", "shares": 1000, "avg_buy_price": 9200, "bought_at": "2026-03-14", "note": "lot 10"},
    {"symbol": "AAPL.US", "shares": 5, "avg_buy_price": 175.5, "bought_at": "2025-11-02"},
    {"symbol": "TLKM", "shares": 500}
  ]
}"#;
    std::fs::write(dir.join("portfolio.json"), body).unwrap();
}

struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_server(dir: &Path) -> Server {
    let port = free_port();
    let log = std::fs::File::create(dir.join("server.log")).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"));
    cmd.env("HQ_BIND", "127.0.0.1")
        .env("HQ_PORT", port.to_string())
        .env("HQ_DATA", dir)
        .env("HQ_WEB", dir.join("web"))
        .env("HQ_LLM", "off")
        .env("HQ_INTERVAL", "3600")
        .env("HQ_STOCKS_FAKE_WIB_DATE", FAKE_DATE)
        .env("HQ_STOCKS_FAKE_WIB_TIME", "16:00")
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log));
    let child = cmd.spawn().expect("binary HQ harus bisa dijalankan");
    let srv = Server { child, port };
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        if let Ok((200, _)) = http_req(srv.port, "GET", "/api/health", None) {
            return srv;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    let log = std::fs::read_to_string(dir.join("server.log")).unwrap_or_default();
    panic!("server tidak siap; log:\n{log}");
}

fn http_req(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let (len, ctype) = match body {
        Some(b) => (b.len(), "Content-Type: application/json\r\n"),
        None => (0, ""),
    };
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{ctype}Content-Length: {len}\r\nConnection: close\r\n\r\n{}",
        body.unwrap_or("")
    );
    stream.write_all(req.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).to_string();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let resp_body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    Ok((status, resp_body))
}

fn get_json(srv: &Server, path: &str) -> Value {
    let (status, body) = http_req(srv.port, "GET", path, None).unwrap();
    assert_eq!(status, 200, "GET {path} -> {body}");
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("GET {path}: {e}: {body}"))
}

fn call(srv: &Server, method: &str, path: &str, body: Option<&str>) -> (u16, Value) {
    let (status, raw) = http_req(srv.port, method, path, body).unwrap();
    let v: Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{method} {path}: {e}: {raw}"));
    (status, v)
}

fn approx(v: &Value, want: f64) -> bool {
    v.as_f64().is_some_and(|x| (x - want).abs() < 0.01)
}

fn holding<'a>(d: &'a Value, sym: &str) -> &'a Value {
    d["holdings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["symbol"] == sym)
        .unwrap_or_else(|| panic!("holding {sym} tidak ada: {d}"))
}

fn total<'a>(d: &'a Value, cur: &str) -> &'a Value {
    d["totals"]["per_currency"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["currency"] == cur)
        .unwrap_or_else(|| panic!("total {cur} tidak ada: {d}"))
}

#[test]
fn portfolio_flow() {
    let dir = std::env::temp_dir().join(format!("hq-porto-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("web")).unwrap();
    write_stocks(&dir);
    write_cache(&dir, FAKE_DATE);
    write_portfolio(&dir);
    // A5: checksum (byte-identik) sebelum operasi portofolio.
    let stocks_before = std::fs::read(dir.join("stocks.json")).unwrap();
    let cache_before = std::fs::read(dir.join("stocks_cache.json")).unwrap();

    let srv = spawn_server(&dir);

    // A1/A2: hitung benar + jujur saat data kosong.
    let d = get_json(&srv, "/api/portfolio");
    let bbca = holding(&d, "BBCA");
    assert!(approx(&bbca["value"], 9_500_000.0), "BBCA value: {bbca}");
    assert!(approx(&bbca["cost"], 9_200_000.0));
    assert!(approx(&bbca["pnl"], 300_000.0));
    assert!(approx(&bbca["pnl_pct"], 3.260869565));
    assert!(approx(&bbca["day_pnl"], 100_000.0));
    assert!(approx(&bbca["day_pnl_pct"], 1.063829787));
    assert!(approx(&bbca["lots"], 10.0));
    let aapl = holding(&d, "AAPL.US");
    assert!(approx(&aapl["value"], 900.0));
    assert!(approx(&aapl["pnl"], 22.5));
    assert!(approx(&aapl["day_pnl"], 10.0));
    let tlkm = holding(&d, "TLKM");
    assert!(
        tlkm["value"].is_null(),
        "TLKM tanpa cache: value harus null, bukan 0: {tlkm}"
    );
    assert!(tlkm["pnl"].is_null());
    assert!(tlkm["day_pnl"].is_null());
    assert!(tlkm["live"].is_null());
    let idr = total(&d, "IDR");
    assert!(approx(&idr["value"], 9_500_000.0));
    assert!(
        idr["cost"].is_null(),
        "total IDR cost null (TLKM tanpa avg_buy_price)"
    );
    assert!(idr["pnl"].is_null());
    let usd = total(&d, "USD");
    assert!(approx(&usd["value"], 900.0));
    assert!(approx(&usd["pnl"], 22.5));
    assert!(
        d["totals"]["grand_idr"].is_null(),
        "tanpa kurs: tidak ada grand total"
    );
    assert!(d["notes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n.as_str().unwrap_or("").contains("belum termasuk dividen")));

    // Kurs manual → grand total IDR muncul (A2 lanjutan).
    let (st, v) = call(
        &srv,
        "POST",
        "/api/portfolio/config",
        Some(&json!({"usd_idr_rate": 16000, "rate_asof": FAKE_DATE}).to_string()),
    );
    assert_eq!(st, 200, "POST config -> {v}");
    let d = get_json(&srv, "/api/portfolio");
    assert!(
        approx(&d["totals"]["grand_idr"], 23_900_000.0),
        "grand: {d}"
    );
    assert_eq!(d["totals"]["fx"]["rate"], json!(16000.0));

    // A4: validasi menolak dengan pesan jelas.
    let (st, v) = call(
        &srv,
        "POST",
        "/api/portfolio/holdings",
        Some(&json!({"symbol": "GOTO", "shares": 100}).to_string()),
    );
    assert_eq!(st, 400, "simbol di luar stocks.json -> {v}");
    assert!(v["error"].as_str().unwrap().contains("stocks.json"));
    let (st, _) = call(
        &srv,
        "POST",
        "/api/portfolio/holdings",
        Some(&json!({"symbol": "BBCA", "shares": 0}).to_string()),
    );
    assert_eq!(st, 400, "shares 0 ditolak");
    let (st, _) = call(
        &srv,
        "POST",
        "/api/portfolio/holdings",
        Some(&json!({"symbol": "TLKM", "shares": 100, "avg_buy_price": 0}).to_string()),
    );
    assert_eq!(st, 400, "avg_buy_price 0 ditolak");
    let (st, _) = call(
        &srv,
        "POST",
        "/api/portfolio/holdings",
        Some(&json!({"symbol": "TLKM", "shares": 100, "bought_at": "2026-11-07"}).to_string()),
    );
    assert_eq!(st, 400, "bought_at masa depan ditolak");
    let (st, _) = call(
        &srv,
        "POST",
        "/api/portfolio/holdings",
        Some(&json!({"symbol": "BBCA", "lots": 1}).to_string()),
    );
    assert_eq!(st, 409, "POST ganda simbol sama -> 409");

    // PATCH ubah jumlah (lot) + DELETE satu holding.
    let (st, v) = call(
        &srv,
        "PATCH",
        "/api/portfolio/holdings/BBCA",
        Some(&json!({"lots": 11}).to_string()),
    );
    assert_eq!(st, 200, "PATCH BBCA -> {v}");
    assert!(approx(&v["holding"]["shares"], 1100.0));
    let (st, _) = call(&srv, "DELETE", "/api/portfolio/holdings/TLKM", None);
    assert_eq!(st, 200, "DELETE TLKM");
    let d = get_json(&srv, "/api/portfolio");
    assert_eq!(d["holdings"].as_array().unwrap().len(), 2);
    assert!(approx(&holding(&d, "BBCA")["value"], 10_450_000.0));

    // A5: stocks.json + stocks_cache.json TIDAK BERUBAH setelah semua operasi.
    assert_eq!(
        std::fs::read(dir.join("stocks.json")).unwrap(),
        stocks_before
    );
    assert_eq!(
        std::fs::read(dir.join("stocks_cache.json")).unwrap(),
        cache_before
    );

    // A3: snapshot idempotent + jujur.
    let (st, v) = call(&srv, "POST", "/api/portfolio/snapshot", None);
    assert_eq!(st, 200, "POST snapshot -> {v}");
    assert_eq!(v["snapshotted"], json!(true));
    assert_eq!(v["date"], json!(FAKE_DATE));
    let h = get_json(&srv, "/api/portfolio/history?days=30");
    assert_eq!(
        h["history"].as_array().unwrap().len(),
        2,
        "1 baris per mata uang: {h}"
    );
    let (st, v) = call(&srv, "POST", "/api/portfolio/snapshot", None);
    assert_eq!(st, 200, "snapshot ke-2 -> {v}");
    let h = get_json(&srv, "/api/portfolio/history?days=30");
    assert_eq!(
        h["history"].as_array().unwrap().len(),
        2,
        "tetap 2 baris (idempotent): {h}"
    );
    // Hari tanpa fetch → TIDAK ADA baris baru.
    write_cache(&dir, "2026-11-05");
    let (st, v) = call(&srv, "POST", "/api/portfolio/snapshot", None);
    assert_eq!(st, 200, "snapshot hari tanpa fetch -> {v}");
    assert_eq!(v["snapshotted"], json!(false));
    let h = get_json(&srv, "/api/portfolio/history?days=30");
    assert_eq!(
        h["history"].as_array().unwrap().len(),
        2,
        "tidak ada baris baru: {h}"
    );
    let h = get_json(&srv, "/api/portfolio/history?days=30&currency=IDR");
    assert_eq!(
        h["history"].as_array().unwrap().len(),
        1,
        "filter currency=IDR: {h}"
    );

    drop(srv);
    let _ = std::fs::remove_dir_all(&dir);
}
