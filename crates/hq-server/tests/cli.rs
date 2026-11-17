use std::process::Command;

#[test]
fn collect_once_json_returns_probe_snapshot_without_database_setup() {
    let output = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"))
        .args(["collect", "--once", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value["processes"].is_array());
    assert!(value["events"].is_array());
}

#[test]
fn device_add_prints_pairing_code_once_and_stores_only_hash() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"))
        .args(["device", "add", "--name", "hp", "--scope", "read"])
        .env("HQ_DATA", dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = stdout
        .split_whitespace()
        .find(|word| {
            word.len() == 8
                && word
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        })
        .expect("pairing code printed once")
        .to_string();
    let db = std::fs::read(dir.path().join("hq.db")).unwrap();
    assert!(
        !db.windows(code.len()).any(|w| w == code.as_bytes()),
        "plaintext pairing code must not be stored"
    );

    let list = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"))
        .args(["device", "list"])
        .env("HQ_DATA", dir.path())
        .output()
        .unwrap();
    assert!(list.status.success());
    assert!(!String::from_utf8_lossy(&list.stdout).contains(&code));
}

#[test]
fn openapi_writes_document_and_checks_schemas() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("openapi.json");
    let output = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"))
        .args(["openapi", "--out", out.to_str().unwrap(), "--check"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(doc["paths"]["/api/v1/overview"]["get"]["responses"]["200"].is_object());
    assert!(doc["components"]["schemas"]["Overview"].is_object());
}

fn fake_state_server(body: &'static str) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (url, handle)
}

#[test]
fn parity_cli_compares_subset_without_destructive_action() {
    let (oracle, oracle_thread) =
        fake_state_server(r#"{"agents":[{"id":"omp","status":"bekerja"}]}"#);
    let (candidate, candidate_thread) =
        fake_state_server(r#"{"agents":[{"id":"agent:omp","status":"bekerja"}]}"#);
    let aliases = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/parity/aliases.json"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"))
        .args([
            "parity",
            "--oracle",
            oracle.as_str(),
            "--candidate",
            candidate.as_str(),
            "--subset",
            "omp,cursor,muse",
            "--duration",
            "0",
            "--aliases",
            aliases,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["verdict"], "PASS");
    assert_eq!(report["status_match_pct"], 100.0);
    assert_eq!(
        report["missing_from_candidate"].as_array().unwrap().len(),
        0
    );
    oracle_thread.join().unwrap();
    candidate_thread.join().unwrap();
}
