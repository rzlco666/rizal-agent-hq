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
        .find(|word| word.len() == 8 && word.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
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
