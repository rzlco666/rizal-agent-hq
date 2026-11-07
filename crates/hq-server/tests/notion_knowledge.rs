//! Uji end-to-end modul knowledge Notion: binary asli + fixture mock.
//!
//! Tidak pernah memakai token asli (tidak ada NOTION_API_KEY di env) dan
//! tidak pernah memanggil Notion asli — mode mock memakai 3 fixture yang
//! di-embed ke binary. Server dijalankan dengan data dir & port sementara;
//! layanan live tidak disentuh. Satu server kedua (mode live dengan base
//! mati) membuktikan sync yang gagal tidak menjatuhkan server.

use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const HQ_DOC: &str = "00000000-0000-4000-8000-000000000001";
const SS_DOC: &str = "00000000-0000-4000-8000-000000000002";
const TRAP_DOC: &str = "00000000-0000-4000-8000-000000000003";

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
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

fn spawn_server(dir: &Path, extra_env: &[(&str, &str)], remove_key: bool) -> Server {
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
    if remove_key {
        cmd.env_remove("NOTION_API_KEY");
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
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

fn fresh_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("web")).unwrap();
    dir
}

fn top_doc_id(search: &Value) -> Option<String> {
    search["results"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r["id"].as_str())
        .map(|s| s.to_string())
}

/// Tunggu sampai sync startup selesai (status menunjukkan last_sync_at),
/// maks 10 detik — menghindari race antara tick pertama scheduler dan test.
fn wait_for_first_sync(srv: &Server) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let st = get_json(srv, "/api/knowledge/sync/status");
        if st["last_sync_at"].as_i64().unwrap_or(0) > 0 {
            return st;
        }
        if Instant::now() > deadline {
            panic!("sync pertama tidak selesai dalam 10 detik: {st}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// POST /api/knowledge/sync, ulangi bila masih ada sync lain yang berjalan
/// (startup sync) — maks 30 detik.
fn post_sync(srv: &Server) -> (u16, Value) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let (code, rep) = call(srv, "POST", "/api/knowledge/sync", None);
        if rep.get("started") == Some(&Value::Bool(true)) {
            return (code, rep);
        }
        if Instant::now() > deadline {
            panic!("POST /api/knowledge/sync tidak pernah start: {rep}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Tunggu sampai baris sync muncul di log server, maks 10 detik.
fn wait_for_log_line(dir: &Path, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let log = std::fs::read_to_string(dir.join("server.log")).unwrap_or_default();
        if log.contains(needle) {
            return;
        }
        if Instant::now() > deadline {
            panic!("baris log tidak muncul dalam 10 detik: {needle}\nlog:\n{log}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
fn mock_sync_end_to_end() {
    let dir = fresh_dir("hq-notion-test");
    let srv = spawn_server(&dir, &[("HQ_NOTION_MOCK", "1")], true);

    // Backfill startup (tick pertama scheduler) sudah jalan saat server siap:
    // tunggu status menunjukkan sync selesai, lalu buktikan di log server.
    // Angka 9/2/3: fallback sumber ada 3 (hq/strutstack/tracerio) dan mock
    // mengembalikan korpus 3 fixture yang sama per sumber — 9 dicek, 2
    // tersimpan (dedup lewat content_hash), 3 karantina (jebakan rahasia
    // diproses sekali per sumber). Di produksi tiap sumber punya root page
    // berbeda sehingga tidak ada duplikasi seperti ini.
    wait_for_first_sync(&srv);
    wait_for_log_line(
        &dir,
        "notion: sync mode=mock checked=9 updated=2 quarantined=3 failures=0",
    );

    // Status: mode mock, 3 halaman, 1 karantina.
    let st = get_json(&srv, "/api/knowledge/sync/status");
    assert_eq!(st["mode"], "mock");
    assert_eq!(st["configured"], false);
    assert_eq!(st["total_pages"], 3);
    assert_eq!(st["quarantined"], 1);
    assert!(st["last_sync_at"].as_i64().unwrap_or(0) > 0);

    // Daftar halaman: 2 synced + 1 quarantined.
    let pages = get_json(&srv, "/api/knowledge/pages");
    let rows = pages["pages"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let by_id = |id: &str| rows.iter().find(|r| r["page_id"] == id).unwrap();
    assert_eq!(by_id(HQ_DOC)["status"], "synced");
    assert_eq!(by_id(SS_DOC)["status"], "synced");
    assert_eq!(by_id(TRAP_DOC)["status"], "quarantined");

    // Sync manual kedua = incremental no-op (tidak ada yang berubah).
    let (code, rep) = post_sync(&srv);
    assert_eq!(code, 200);
    assert_eq!(rep["pages_checked"], 0);
    assert_eq!(rep["pages_updated"], 0);
    assert_eq!(rep["mode"], "mock");

    // A3: 0 hasil pencarian untuk pola rahasia fixture jebakan.
    let trap = get_json(&srv, "/api/knowledge/search?q=sk-ant-abc1234567890");
    assert_eq!(trap["results"].as_array().unwrap().len(), 0);
    let trap2 = get_json(&srv, "/api/knowledge/search?q=Jebakan");
    assert_eq!(trap2["results"].as_array().unwrap().len(), 0);

    // Satu dokumen lengkap + provenance.
    let doc = get_json(&srv, &format!("/api/knowledge/page/{HQ_DOC}"));
    assert_eq!(doc["title"], "Status saat ini — Rizal Agent HQ");
    assert!(doc["content_text"].as_str().unwrap().contains("both-pass"));
    assert_eq!(doc["group"], "hq");
}

#[test]
fn mock_search_precision_probes() {
    let dir = fresh_dir("hq-notion-probe");
    let srv = spawn_server(&dir, &[("HQ_NOTION_MOCK", "1")], true);
    wait_for_first_sync(&srv);
    // 5 probe query (A2): dokumen yang benar harus ada di 5 teratas.
    let probes = [
        ("both-pass", HQ_DOC),
        ("EODHD", HQ_DOC),
        ("0065", HQ_DOC),
        ("toolchain 1.99", HQ_DOC),
        ("tracer study", HQ_DOC),
        ("FORCE RLS", SS_DOC),
        ("StrutStack V2", SS_DOC),
    ];
    for (q, want) in probes {
        let res = get_json(&srv, &format!("/api/knowledge/search?q={}", url_encode(q)));
        let ids: Vec<String> = res["results"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|r| r["id"].as_str().map(|s| s.to_string()))
            .collect();
        assert!(
            ids.iter().take(5).any(|id| id == want),
            "probe {q:?}: {want} tidak di 5 teratas: {ids:?}"
        );
    }
    // Urutan: trusted memory dulu — belum ada memori, jadi doc pertama.
    let res = get_json(&srv, "/api/knowledge/search?q=EODHD");
    assert_eq!(top_doc_id(&res).as_deref(), Some(HQ_DOC));
    // Halaman karantina tidak boleh muncul di pencarian — bahkan lewat judul
    // (rebuild FTS mengindeks judulnya, tapi pencarian hanya menyajikan
    // dokumen berstatus synced).
    let trap = get_json(&srv, "/api/knowledge/search?q=kunci%20cadangan");
    assert!(
        trap["results"].as_array().unwrap().is_empty(),
        "dokumen karantina bocor ke pencarian: {trap}"
    );
}

fn url_encode(s: &str) -> String {
    s.replace(' ', "%20")
}

#[test]
fn zero_auto_trusted_dan_promote_flow() {
    let dir = fresh_dir("hq-notion-promote");
    let srv = spawn_server(&dir, &[("HQ_NOTION_MOCK", "1")], true);
    wait_for_first_sync(&srv);

    // A4: setelah sync penuh, 0 entri memori — sync tidak menulis memori.
    let mem = get_json(&srv, "/api/memory");
    assert_eq!(mem["entries"].as_array().unwrap().len(), 0);

    // Promosi dari dokumen -> candidate (bukan trusted).
    let (code, out) = call(
        &srv,
        "POST",
        "/api/knowledge/promote",
        Some(&format!(r#"{{"page_id": "{HQ_DOC}"}}"#)),
    );
    assert_eq!(code, 201, "{out}");
    assert_eq!(out["status"], "candidate");
    let id = out["id"].as_str().unwrap().to_string();

    // Dokumen karantina tidak bisa dipromosikan.
    let (code, _) = call(
        &srv,
        "POST",
        "/api/knowledge/promote",
        Some(&format!(r#"{{"page_id": "{TRAP_DOC}"}}"#)),
    );
    assert_eq!(code, 400);

    // Candidate -> trusted lewat /api/memory/:id/promote.
    let (code, out) = call(
        &srv,
        "POST",
        &format!("/api/memory/{id}/promote"),
        Some(r#"{"actor": "melo"}"#),
    );
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["status"], "trusted");

    // Tulis trusted langsung oleh agen DITOLAK.
    let (code, _) = call(
        &srv,
        "POST",
        "/api/memory",
        Some(
            r#"{"kind": "semantic", "scope": "global", "title": "X", "body": "isi", "status": "trusted", "created_by": "agent:dev"}"#,
        ),
    );
    assert_eq!(code, 400);

    // Teks berahasia DITOLAK (bukan disimpan tersensor).
    let (code, _) = call(
        &srv,
        "POST",
        "/api/memory",
        Some(
            r#"{"kind": "semantic", "scope": "global", "title": "Kunci", "body": "token=abc123", "created_by": "melo"}"#,
        ),
    );
    assert_eq!(code, 400);

    // Reject butuh alasan; correct membuat rantai supersession.
    let (code, out) = call(
        &srv,
        "POST",
        "/api/memory",
        Some(
            r#"{"kind": "semantic", "scope": "global", "title": "Dua", "body": "isi dua", "created_by": "melo"}"#,
        ),
    );
    assert_eq!(code, 201);
    let id2 = out["id"].as_str().unwrap().to_string();
    let (code, _) = call(
        &srv,
        "POST",
        &format!("/api/memory/{id2}/reject"),
        Some(r#"{"actor": "melo", "reason": ""}"#),
    );
    assert_eq!(code, 400);
    let (code, _) = call(
        &srv,
        "POST",
        &format!("/api/memory/{id2}/reject"),
        Some(r#"{"actor": "melo", "reason": "tidak akurat"}"#),
    );
    assert_eq!(code, 200);
}

#[test]
fn sync_gagal_tidak_menjatuhkan_server() {
    // Mode live dengan base mati: sync gagal (jaringan), server tetap hidup.
    let dir = fresh_dir("hq-notion-fail");
    let srv = spawn_server(
        &dir,
        &[
            ("NOTION_API_KEY", "fake-test-key"),
            ("NOTION_API_BASE", "http://127.0.0.1:9"),
        ],
        false,
    );
    let (code, body) = http_req(srv.port, "GET", "/api/health", None).unwrap();
    assert_eq!(code, 200);
    assert!(body.contains("\"ok\":true"));

    // Startup sync live gagal (base mati) — tunggu sampai laporannya ada,
    // lalu sync manual harus melaporkan kegagalan yang sama.
    let (code, rep) = post_sync(&srv);
    assert_eq!(code, 200);
    assert_eq!(rep["synced"], false);
    assert_eq!(rep["mode"], "live");
    assert!(!rep["failures"].as_array().unwrap().is_empty());

    let st = get_json(&srv, "/api/knowledge/sync/status");
    assert_eq!(st["mode"], "live");
    assert_eq!(st["configured"], true);
    assert_eq!(st["total_pages"], 0);
}
