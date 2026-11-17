use std::process::Command;

#[test]
fn binary_reports_v3_version() {
    let binary = env!("CARGO_BIN_EXE_rizal-agent-hq");
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .expect("binary must be runnable");

    assert!(
        output.status.success(),
        "binary exited with {:?}",
        output.status
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "rizal-agent-hq 3.0.0\n"
    );
}
