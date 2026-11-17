//! Uji end-to-end harga saham live: binary asli + mock EODHD/FMP di 127.0.0.1.
//!
//! Tidak pernah memakai kunci asli — hanya literal "PLACEHOLDER-TEST-KEY".
//! Server dijalankan dengan data dir & port sementara; layanan live tidak disentuh.

use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TEST_KEY: &str = "PLACEHOLDER-TEST-KEY";
/// 2026-11-05 15:01 WIB — tanggal bursa yang diharapkan dari balasan Live.
const LIVE_TS: i64 = 1_793_865_660;
const WEEKDAY: &str = "2026-11-06";
const MARKET_DATE: &str = "2026-11-05";
const SATURDAY: &str = "2026-11-07";

// --------------------------------------------------------------- mock server

struct Mock {
    port: u16,
    hits: Arc<Mutex<Vec<String>>>,
}

impl Mock {
    fn start(mode: u8) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(Mutex::new(Vec::new()));
        let mode = Arc::new(AtomicU8::new(mode));
        let (h, m) = (hits.clone(), mode.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                handle(&mut stream, &h, &m);
            }
        });
        Mock { port, hits }
    }

    fn base(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn hits(&self) -> Vec<String> {
        self.hits.lock().unwrap().clone()
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let resp = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

fn handle(stream: &mut TcpStream, hits: &Arc<Mutex<Vec<String>>>, mode: &Arc<AtomicU8>) {
    let req = read_request(stream);
    let path = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("")
        .to_string();
    hits.lock().unwrap().push(path.clone());
    if path.starts_with("/api/user") {
        respond(
            stream,
            200,
            r#"{"apiRequests":2,"dailyRateLimit":20,"extraLimit":500}"#,
        );
    } else if path.starts_with("/api/real-time/") {
        if mode.load(Ordering::SeqCst) == 1 {
            respond(stream, 402, r#"{"error":"quota reached"}"#);
            return;
        }
        // Mode 2: fixture .US — EODHD Live mengembalikan simbol US
        // (penutupan Jumat 2026-11-06, terverifikasi ke API asli).
        let body = if mode.load(Ordering::SeqCst) == 2 {
            format!(
                r#"{{"BBCA.JK":{{"code":"BBCA.JK","timestamp":{LIVE_TS},"close":5025,"previousClose":5000}},"NVDA.US":{{"code":"NVDA.US","timestamp":{LIVE_TS},"close":186.64,"previousClose":181.6}}}}"#
            )
        } else {
            format!(
                r#"{{"BBCA.JK":{{"code":"BBCA.JK","timestamp":{LIVE_TS},"close":5025,"previousClose":5000}},"TLKM.JK":{{"code":"TLKM.JK","timestamp":{LIVE_TS},"close":3100}}}}"#
            )
        };
        respond(stream, 200, &body);
    } else if path.starts_with("/api/eod/") {
        respond(
            stream,
            200,
            r#"[{"date":"2026-11-03","close":4100},{"date":"2026-11-04","close":4150}]"#,
        );
    } else if path.starts_with("/stable/quote") {
        respond(
            stream,
            200,
            r#"[{"symbol":"SAP.DE","price":190.5,"exchange":"XETRA"}]"#,
        );
    } else {
        respond(stream, 404, "{}");
    }
}

// ------------------------------------------------------------ server binary

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

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hq-stocks-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_stocks(dir: &Path) {
    let body = r#"[
  {"symbol": "BBCA", "name": "Bank Central Asia", "shares": 100, "price": 9000, "currency": "IDR"},
  {"symbol": "TLKM", "name": "Telkom Indonesia", "shares": 500, "price": 3100, "currency": "IDR"},
  {"symbol": "SAP.DE", "name": "SAP", "shares": 2, "price": 180, "currency": "EUR"}
]"#;
    std::fs::write(dir.join("stocks.json"), body).unwrap();
}

fn spawn_server(dir: &Path, envs: &[(&str, String)]) -> Server {
    let port = free_port();
    let log = std::fs::File::create(dir.join("server.log")).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"));
    cmd.env("HQ_BIND", "127.0.0.1")
        .env("HQ_PORT", port.to_string())
        .env("HQ_DATA", dir)
        .env("HQ_WEB", dir.join("web"))
        .env("HQ_LLM", "off")
        .env("HQ_INTERVAL", "3600")
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log));
    for (k, v) in envs {
        cmd.env(*k, v);
    }
    let child = cmd.spawn().expect("binary HQ harus bisa dijalankan");
    let srv = Server { child, port };
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        if let Ok((200, _)) = http(srv.port, "GET", "/api/health") {
            return srv;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    let log = std::fs::read_to_string(dir.join("server.log")).unwrap_or_default();
    panic!("server tidak siap; log:\n{log}");
}

fn http(port: u16, method: &str, path: &str) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
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
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    Ok((status, body))
}

fn get_json(port: u16, path: &str) -> Value {
    let (status, body) = http(port, "GET", path).unwrap();
    assert_eq!(status, 200, "GET {path} -> {body}");
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("GET {path}: {e}: {body}"))
}

fn post_json(port: u16, path: &str) -> Value {
    let (status, body) = http(port, "POST", path).unwrap();
    assert_eq!(status, 200, "POST {path} -> {body}");
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("POST {path}: {e}: {body}"))
}

fn keyed_env(mock: &Mock, fake_date: &str, fake_time: &str) -> Vec<(&'static str, String)> {
    vec![
        ("EODHD_API_KEY", TEST_KEY.to_string()),
        ("FMP_API_KEY", TEST_KEY.to_string()),
        ("EODHD_BASE_URL", mock.base()),
        ("FMP_BASE_URL", mock.base()),
        ("HQ_STOCKS_FAKE_WIB_DATE", fake_date.to_string()),
        ("HQ_STOCKS_FAKE_WIB_TIME", fake_time.to_string()),
    ]
}

// ---------------------------------------------------------------------- uji

#[test]
fn tanpa_kunci_panel_harga_file_tanpa_permintaan() {
    let mock = Mock::start(0);
    let dir = tmp_dir("nokey");
    write_stocks(&dir);
    let before = std::fs::read(dir.join("stocks.json")).unwrap();
    let srv = spawn_server(
        &dir,
        &[
            ("EODHD_API_KEY", String::new()),
            ("FMP_API_KEY", String::new()),
            ("EODHD_BASE_URL", mock.base()),
            ("FMP_BASE_URL", mock.base()),
        ],
    );

    let d = get_json(srv.port, "/api/stocks");
    assert_eq!(d["stocks"].as_array().unwrap().len(), 3);
    assert_eq!(d["source"], "file");
    assert!(d["error"].is_null());
    assert_eq!(d["quotes"].as_object().unwrap().len(), 0);
    assert!(d["quotes_updated_at"].is_null());

    let r = post_json(srv.port, "/api/stocks/refresh");
    assert_eq!(r["refreshed"], false);
    assert!(r["reason"]
        .as_str()
        .unwrap()
        .contains("belum dikonfigurasi"));

    assert!(
        mock.hits().is_empty(),
        "tanpa kunci tidak boleh ada permintaan keluar"
    );
    assert_eq!(std::fs::read(dir.join("stocks.json")).unwrap(), before);
    assert!(!dir.join("stocks_cache.json").exists());
}

#[test]
fn satu_kali_refresh_mengisi_cache_lalu_gerbang_harian() {
    let mock = Mock::start(0);
    let dir = tmp_dir("keyed");
    write_stocks(&dir);
    let before = std::fs::read(dir.join("stocks.json")).unwrap();
    // 09:00 WIB: penjadwal diam (sebelum jendela), trigger manual tetap boleh.
    let srv = spawn_server(&dir, &keyed_env(&mock, WEEKDAY, "09:00"));

    let r = post_json(srv.port, "/api/stocks/refresh");
    assert_eq!(r["refreshed"], true, "{r}");
    assert!(r["reason"].as_str().unwrap().contains("3 simbol"), "{r}");

    let d = get_json(srv.port, "/api/stocks");
    let quotes = d["quotes"].as_object().unwrap();
    assert_eq!(quotes.len(), 3);
    let bbca = &quotes["BBCA"];
    assert_eq!(bbca["price"], 5025.0);
    assert_eq!(bbca["source"], "EODHD-Live");
    assert_eq!(bbca["asof"], MARKET_DATE);
    assert_eq!(bbca["stale"], false);
    assert_eq!(quotes["TLKM"]["price"], 3100.0);
    // SAP.DE tetap lewat FMP (sufiks non-EODHD); .US kini lewat EODHD.
    let sap = &quotes["SAP.DE"];
    assert_eq!(sap["price"], 190.5);
    assert_eq!(sap["source"], "FMP");
    assert!(d["quotes_updated_at"]
        .as_str()
        .unwrap()
        .starts_with(WEEKDAY));

    // Harga file & total tidak berubah; berkas tidak ditulis ulang.
    assert_eq!(d["stocks"][0]["price"], 9000.0);
    assert_eq!(std::fs::read(dir.join("stocks.json")).unwrap(), before);

    // Kuota: 1 panggilan per simbol (3) + 1 pemeriksaan kuota gratis (0).
    let hits = mock.hits();
    let count = |p: &str| hits.iter().filter(|h| h.starts_with(p)).count();
    assert_eq!(count("/api/user"), 1);
    assert_eq!(count("/api/real-time/"), 1);
    assert_eq!(count("/stable/quote"), 1);
    assert_eq!(count("/api/eod/"), 0);
    assert_eq!(hits.len(), 3);
    for h in &hits {
        assert!(
            !(h.starts_with("/stable/quote") && h.contains(".JK")),
            "FMP tidak boleh dipanggil dengan simbol .JK: {h}"
        );
        assert!(
            !(h.starts_with("/api/real-time/") && h.contains("SAP")),
            "EODHD tidak boleh dipanggil untuk simbol .DE: {h}"
        );
    }
    assert!(hits
        .iter()
        .any(|h| h.starts_with("/stable/quote?symbol=SAP.DE")));

    // Cache: tanpa kunci, gerbang tercatat.
    let cache = std::fs::read_to_string(dir.join("stocks_cache.json")).unwrap();
    assert!(!cache.contains(TEST_KEY), "kunci tidak boleh masuk cache");
    let cache_json: Value = serde_json::from_str(&cache).unwrap();
    assert_eq!(cache_json["last_fetch_wib_date"], WEEKDAY);

    // Refresh kedua di hari yang sama: nol panggilan baru.
    let r2 = post_json(srv.port, "/api/stocks/refresh");
    assert_eq!(r2["refreshed"], false);
    assert!(r2["reason"]
        .as_str()
        .unwrap()
        .contains("sudah diambil hari ini"));
    assert_eq!(mock.hits().len(), 3);
}

#[test]
fn us_lewat_eodhd_live_bukan_fmp() {
    // Routing baru: .US lewat EODHD Live (paket gratis mencakup US),
    // bukan FMP (paket saat ini 402). Mock mode 2 mengembalikan NVDA.US
    // dari "EODHD" dengan penutupan Jumat 2026-11-06 yang terverifikasi.
    let mock = Mock::start(2);
    let dir = tmp_dir("us");
    std::fs::write(
        dir.join("stocks.json"),
        r#"[{"symbol": "BBCA", "shares": 100, "price": 9000, "currency": "IDR"},
            {"symbol": "NVDA.US", "shares": 2, "price": 180, "currency": "USD"}]"#,
    )
    .unwrap();
    let srv = spawn_server(&dir, &keyed_env(&mock, WEEKDAY, "09:00"));

    let r = post_json(srv.port, "/api/stocks/refresh");
    assert_eq!(r["refreshed"], true, "{r}");
    assert!(r["reason"].as_str().unwrap().contains("2 simbol"), "{r}");

    let d = get_json(srv.port, "/api/stocks");
    let quotes = d["quotes"].as_object().unwrap();
    assert_eq!(quotes.len(), 2);
    assert_eq!(quotes["BBCA"]["price"], 5025.0);
    assert_eq!(quotes["BBCA"]["source"], "EODHD-Live");
    let nvda = &quotes["NVDA.US"];
    assert_eq!(nvda["price"], 186.64);
    assert_eq!(nvda["source"], "EODHD-Live");

    // Kuota: 1 pemeriksaan kuota (gratis) + 1 batch Live; FMP tidak disentuh.
    let hits = mock.hits();
    let count = |p: &str| hits.iter().filter(|h| h.starts_with(p)).count();
    assert_eq!(count("/api/user"), 1);
    assert_eq!(count("/api/real-time/"), 1);
    assert_eq!(count("/stable/quote"), 0);
    assert_eq!(count("/api/eod/"), 0);
    assert_eq!(hits.len(), 2);
}

#[test]
fn akhir_pekan_tidak_mengambil() {
    let mock = Mock::start(0);
    let dir = tmp_dir("weekend");
    write_stocks(&dir);
    let srv = spawn_server(&dir, &keyed_env(&mock, SATURDAY, "15:05"));

    let r = post_json(srv.port, "/api/stocks/refresh");
    assert_eq!(r["refreshed"], false);
    assert!(r["reason"].as_str().unwrap().contains("akhir pekan"), "{r}");
    assert!(mock.hits().is_empty());
    assert!(!dir.join("stocks_cache.json").exists());
}

#[test]
fn kuota_402_memakai_cache_lama_dengan_label_tanggal() {
    let mock = Mock::start(1);
    let dir = tmp_dir("402");
    // Hanya simbol IDX: tidak ada jalur FMP yang bisa "menyelamatkan" refresh.
    std::fs::write(
        dir.join("stocks.json"),
        r#"[{"symbol": "BBCA", "shares": 100, "price": 9000, "currency": "IDR"},
            {"symbol": "TLKM", "shares": 500, "price": 3100, "currency": "IDR"}]"#,
    )
    .unwrap();
    let old = r#"{
  "quotes_updated_at": "2026-11-05T15:01:00+07:00",
  "last_fetch_wib_date": "2026-11-05",
  "symbols": {
    "BBCA": {"symbol":"BBCA","price":5000,"asof":"2026-11-05","fetched_at":"2026-11-05T15:01:00+07:00","source":"EODHD-Live","last_fetch_wib_date":"2026-11-05"}
  }
}"#;
    std::fs::write(dir.join("stocks_cache.json"), old).unwrap();
    let srv = spawn_server(&dir, &keyed_env(&mock, WEEKDAY, "09:00"));

    let r = post_json(srv.port, "/api/stocks/refresh");
    assert_eq!(r["refreshed"], false);
    assert!(r["reason"].as_str().unwrap().contains("kuota"), "{r}");

    let d = get_json(srv.port, "/api/stocks");
    let quote = &d["quotes"]["BBCA"];
    assert_eq!(quote["price"], 5000.0);
    assert_eq!(quote["asof"], "2026-11-05");
    assert_eq!(quote["stale"], true);
    assert_eq!(quote["source"], "EODHD-Live");
    assert!(d["error"].is_null());

    let hits = mock.hits();
    assert_eq!(
        hits.iter()
            .filter(|h| h.starts_with("/api/real-time/"))
            .count(),
        1
    );
    assert_eq!(
        hits.iter().filter(|h| h.starts_with("/api/eod/")).count(),
        0
    );
}

#[test]
fn simbol_berprioritas_terkirim_lebih_dulu_saat_kuota_sempit() {
    let mock = Mock::start(0);
    let dir = tmp_dir("prio");
    // 3 simbol; semua .JK agar satu batch. Kuota mock menyisakan 18 (2 terpakai).
    let body = r#"[
  {"symbol": "BBCA", "shares": 1, "price": 1},
  {"symbol": "TLKM", "shares": 1, "price": 1, "priority": true},
  {"symbol": "ASII", "shares": 1, "price": 1}
]"#;
    std::fs::write(dir.join("stocks.json"), body).unwrap();
    let srv = spawn_server(&dir, &keyed_env(&mock, WEEKDAY, "09:00"));
    let r = post_json(srv.port, "/api/stocks/refresh");
    assert_eq!(r["refreshed"], true, "{r}");
    let hits = mock.hits();
    let live = hits
        .iter()
        .find(|h| h.starts_with("/api/real-time/"))
        .expect("satu request Live batch");
    assert!(live.contains("BBCA.JK"), "{live}");
    assert!(live.contains("s=TLKM.JK,ASII.JK"), "{live}");
    let d = get_json(srv.port, "/api/stocks");
    assert_eq!(d["stocks"][1]["priority"], true);
}
