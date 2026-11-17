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
        assert_ne!(
            res.status(),
            StatusCode::OK,
            "path {path} fell back to index"
        );
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

#[tokio::test]
async fn dev_directory_rejects_path_traversal() {
    use axum::{Router, body::Body, http::Request};
    use tower::ServiceExt;

    let parent = tempfile::tempdir().unwrap();
    let web = parent.path().join("web");
    std::fs::create_dir(&web).unwrap();
    std::fs::write(web.join("index.html"), "<!doctype html><html></html>").unwrap();
    unsafe { std::env::set_var("HQ_WEB_DIR", &web) };

    let app = Router::new().fallback(hq_api::static_assets::serve_asset);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/../secret.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    unsafe { std::env::remove_var("HQ_WEB_DIR") };

    assert_ne!(response.status(), StatusCode::OK);
    assert!(!body_text(response).await.contains("TOP-SECRET"));
}
