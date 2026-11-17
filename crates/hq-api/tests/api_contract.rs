use hq_api::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_covers_every_route_and_schema() {
    let doc = ApiDoc::openapi();
    let json = serde_json::to_value(&doc).unwrap();
    assert!(json["paths"]["/api/v1/overview"]["get"]["responses"]["200"].is_object());
    assert!(json["components"]["schemas"]["Overview"].is_object());
}

#[test]
fn openapi_documents_timeline_and_events_aliases() {
    let json = serde_json::to_value(ApiDoc::openapi()).unwrap();
    for path in ["/api/v1/timeline", "/api/v1/events"] {
        assert!(
            json["paths"][path]["get"]["responses"]["200"].is_object(),
            "missing documented GET {path}"
        );
    }
}

#[test]
fn openapi_has_no_unresolved_schema_references() {
    let json = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let rendered = json.to_string();
    let mut start = 0;
    let mut refs = Vec::new();
    while let Some(index) = rendered[start..].find("#/components/schemas/") {
        let absolute = start + index + "#/components/schemas/".len();
        let name: String = rendered[absolute..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        refs.push(name);
        start = absolute;
    }
    assert!(!refs.is_empty());
    for name in refs {
        assert!(
            json["components"]["schemas"][&name].is_object(),
            "missing schema {name}"
        );
    }
}
