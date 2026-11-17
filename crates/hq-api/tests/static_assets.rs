mod common;

use axum::http::StatusCode;
use common::{body_text, get, loopback, test_app};

#[tokio::test]
async fn serves_embedded_index() {
    let app = test_app().await;
    let res = get(app.app.clone(), "/", loopback()).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_text(res).await.contains("<!doctype html>"));
}

#[tokio::test]
async fn spa_fallback_does_not_shadow_api_or_ws() {
    let app = test_app().await;
    for path in ["/api/v1/unknown", "/ws/unknown", "/events/stream"] {
        let res = get(app.app.clone(), path, loopback()).await;
        assert_ne!(res.status(), StatusCode::OK, "path {path} fell back to index");
        assert!(!body_text(res).await.contains("<!doctype html>"));
    }
}

#[tokio::test]
async fn spa_fallback_serves_index_for_client_routes() {
    let app = test_app().await;
    let res = get(app.app.clone(), "/office", loopback()).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_text(res).await.contains("<!doctype html>"));
}
