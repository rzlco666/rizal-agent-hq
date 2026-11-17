use hq_connectors::parity::{compare_states, parse_aliases, State};
use serde_json::{json, Value};

fn fake_oracle(value: Value) -> State {
    serde_json::from_value(value).unwrap()
}

fn fake_candidate(value: Value) -> State {
    serde_json::from_value(value).unwrap()
}

fn aliases() -> std::collections::HashMap<String, String> {
    parse_aliases(r#"{"omp":"agent:omp","cursor":"agent:cursor","Muse":"agent:muse"}"#).unwrap()
}

#[tokio::test]
async fn subset_parity_reports_missing_agent() {
    let oracle = fake_oracle(json!({"agents":[{"id":"omp","status":"bekerja"}]}));
    let cand = fake_candidate(json!({"agents":[
        {"id":"agent:omp","status":"bekerja"},
        {"id":"agent:cursor","status":"santai"}
    ]}));
    let r = compare_states(&oracle, &cand, &aliases(), &["omp", "cursor", "muse"])
        .await
        .unwrap();
    assert_eq!(r.missing_from_candidate.len(), 0);
    assert_eq!(r.status_match_pct, 100.0);
    assert_eq!(r.extra_in_candidate, vec!["agent:cursor"]);
    assert_eq!(r.verdict, "PASS");
}

#[tokio::test]
async fn oracle_agent_absent_from_candidate_is_reported() {
    let oracle = fake_oracle(json!({"agents":[{"id":"Muse","status":"menunggu"}]}));
    let cand = fake_candidate(json!({"agents":[{"id":"agent:omp","status":"bekerja"}]}));
    let r = compare_states(&oracle, &cand, &aliases(), &["omp", "cursor", "muse"])
        .await
        .unwrap();
    assert_eq!(r.missing_from_candidate, vec!["agent:muse"]);
    assert_eq!(r.status_match_pct, 0.0);
    assert_eq!(r.verdict, "FAIL");
}

#[tokio::test]
async fn usage_is_rounded_to_three_significant_figures() {
    let oracle = fake_oracle(json!({"agents":[{
        "id":"omp","status":"bekerja",
        "usage":{"requests":1000,"input_tokens":123456789,"output_tokens":5000}
    }]}));
    let cand = fake_candidate(json!({"agents":[{
        "id":"agent:omp","status":"bekerja",
        "usage":{"requests":1000,"input_tokens":124000000,"output_tokens":5000}
    }]}));
    let r = compare_states(&oracle, &cand, &aliases(), &["omp"])
        .await
        .unwrap();
    assert!(r.usage_diff_pct > 0.0 && r.usage_diff_pct < 1.0);
    assert_eq!(r.verdict, "PASS");
}

#[tokio::test]
async fn display_timestamps_are_ignored() {
    let oracle = fake_oracle(json!({"agents":[{
        "id":"omp","status":"bekerja","display_ts":"2026-11-18T01:00:00Z"
    }]}));
    let cand = fake_candidate(json!({"agents":[{
        "id":"agent:omp","status":"bekerja","display_ts":"1999-01-01T00:00:00Z"
    }]}));
    let r = compare_states(&oracle, &cand, &aliases(), &["omp"])
        .await
        .unwrap();
    assert_eq!(r.status_match_pct, 100.0);
    assert_eq!(r.usage_diff_pct, 0.0);
}

#[test]
fn frozen_aliases_are_parseable_without_modification() {
    let a = parse_aliases(include_str!("../../../tools/parity/aliases.json")).unwrap();
    assert_eq!(a.get("omp").map(String::as_str), Some("agent:omp"));
    assert_eq!(a.get("Muse").map(String::as_str), Some("agent:muse"));
}
