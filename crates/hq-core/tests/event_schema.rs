use hq_core::*;

#[test]
fn every_kind_has_a_schema() {
    for kind in EventKind::ALL {
        let schema = schema_for(kind, 1).expect("schema");
        assert_eq!(schema["type"], "object");
    }
}

#[test]
fn missing_required_field_is_rejected() {
    let draft = EventDraft::test_with_payload(EventKind::UsageRecorded, serde_json::json!({}));
    assert!(matches!(draft.validate(), Err(CoreError::Schema(_))));
}

#[test]
fn unknown_schema_version_is_rejected() {
    assert!(schema_for(EventKind::AgentObserved, 99).is_err());
}
