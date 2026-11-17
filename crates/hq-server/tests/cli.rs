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
