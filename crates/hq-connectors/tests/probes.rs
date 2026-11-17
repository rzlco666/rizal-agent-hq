use hq_connectors::probes::{parse_ps, run_timeout};
use std::time::{Duration, Instant};

#[test]
fn parses_ps_and_redacts_args() {
    let processes = parse_ps(include_str!("fixtures/ps.txt"));
    assert_eq!(processes.len(), 5);
    assert_eq!(processes[0].ppid, 1);
    assert_eq!(processes[0].args, "omp --model local");
    assert!(
        !processes
            .iter()
            .any(|process| process.args.contains("synthetic-token"))
    );
}

#[test]
fn timeout_kills_hung_command() {
    assert!(run_timeout("sleep", &["5"], 100).is_none());
}

#[test]
fn missing_binary_is_none_not_panic() {
    assert!(run_timeout("hq-no-such-binary", &[], 100).is_none());
}

#[test]
fn descendant_holding_stdout_does_not_block_run_timeout() {
    let started = Instant::now();
    let out = run_timeout("sh", &["-c", "sleep 3 &"], 200);
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "run_timeout blocked {elapsed:?} waiting for descendant stdout"
    );
    assert!(out.is_none());
}

#[test]
fn parses_tmux_sessions() {
    let sessions = hq_connectors::probes::parse_tmux(include_str!("fixtures/tmux.txt"));
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].name, "omp-main");
}

#[test]
fn parses_docker_json_lines() {
    let containers = hq_connectors::probes::parse_docker(include_str!("fixtures/docker.json"));
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].image, "postgres:16");
}

#[test]
fn parses_git_log() {
    let commits = hq_connectors::probes::parse_git(include_str!("fixtures/git.txt"));
    assert_eq!(commits[0].hash.len(), 7);
}

#[test]
fn parses_meminfo_fixture() {
    let percent = hq_connectors::probes::parse_meminfo(include_str!("fixtures/meminfo")).unwrap();
    assert!((percent - 60.0).abs() < f64::EPSILON);
}
