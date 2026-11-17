mod common;

use crate::common::*;

#[tokio::test]
async fn health_reports_db_and_seq() {
    let app = test_app().await;
    let res = get(app.app, "/api/v1/health", loopback()).await;
    assert_eq!(res.status(), 200);
    let j = body_json(res).await;
    assert_eq!(j["ok"], true);
    assert!(j["seq"].as_i64().is_some());
}
