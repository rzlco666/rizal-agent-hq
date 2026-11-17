use hq_core::*;

#[test]
fn corpus_secrets_are_redacted() {
    let input = include_str!("corpus/redaction/secrets.txt");
    let expected = include_str!("corpus/redaction/expected_secrets.txt");
    assert_eq!(redact_text(input), expected);
}

#[test]
fn safe_values_survive() {
    let input = include_str!("corpus/redaction/safe.txt");
    assert_eq!(redact_text(input), input);
}

#[test]
fn redaction_is_idempotent() {
    let once = redact_text(include_str!("corpus/redaction/secrets.txt"));
    assert_eq!(redact_text(&once), once);
}

#[test]
fn redacts_nested_json_strings() {
    let mut v = serde_json::json!({"msg": "token ghp_abcd1234", "n": 1});
    redact_json(&mut v);
    assert!(!v.to_string().contains("ghp_"));
    assert_eq!(v["n"], 1);
}
