use hq_core::*;
use serde_json::json;

#[test]
fn identity_upcast_preserves_payload() {
    let v = json!({"a": 1});
    assert_eq!(upcast(EventKind::AgentObserved, 1, v.clone()).unwrap(), v);
}

#[test]
fn future_version_fails() {
    assert!(upcast(EventKind::AgentObserved, 2, json!({})).is_err());
}
