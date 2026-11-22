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

fn fake_state_server_sequence(bodies: Vec<&'static str>) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        for body in bodies {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    (url, handle)
}

#[test]
fn collect_once_includes_omp_stats_events_from_cold_refresh() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("fake-omp");
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../hq-connectors/tests/fixtures/omp-stats.json"
    );
    std::fs::write(&script, format!("#!/bin/sh\ncat '{fixture}'\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_rizal-agent-hq"))
        .args(["collect", "--once", "--json"])
        .env("HOME", dir.path())
        .env("HQ_DATA", dir.path().join("data"))
        .env("HQ_OMP_STATS_BIN", script.to_string_lossy().into_owned())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let events = value["events"].as_array().unwrap();
    let usage = events
        .iter()
        .find(|event| event["subject"] == "usage:global")
        .expect("OMP stats usage event must be emitted by collect");
    assert_eq!(usage["kind"], "usage_recorded");
    assert_eq!(usage["payload"]["input_tokens"], 108_000_000);
    assert!(
        value["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|agent| agent["id"] == "agent:omp")
    );
}

#[test]
fn parity_cli_fails_when_agent_disappears_in_a_later_sample() {
    let present = r#"{"agents":[{"id":"Muse","status":"bekerja"}]}"#;
    let candidate = r#"{"agents":[{"id":"agent:muse","status":"bekerja"}]}"#;
    let absent = r#"{"agents":[]}"#;
    let (oracle, oracle_thread) = fake_state_server_sequence(vec![present, present, present]);
    let (cand, cand_thread) = fake_state_server_sequence(vec![candidate, absent, absent]);
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
            cand.as_str(),
            "--subset",
            "omp,cursor,muse",
            "--duration",
            "2",
            "--interval",
            "1",
            "--aliases",
            aliases,
        ])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success());
    assert_eq!(report["verdict"], "FAIL");
    assert_eq!(
        report["missing_from_candidate"],
        serde_json::json!(["agent:muse"])
    );
    oracle_thread.join().unwrap();
    cand_thread.join().unwrap();
}
