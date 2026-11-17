mod common;

use crate::common::*;
use axum::http::StatusCode;
use hq_api::AppState;
use hq_store::*;
use tower::ServiceExt;
#[tokio::test]
async fn loopback_is_owner() {
    let app = test_app().await;
    assert_eq!(
        get(app.app, "/api/v1/agents", loopback()).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn remote_without_token_is_401() {
    let app = test_app().await;
    assert_eq!(
        get(app.app, "/api/v1/agents", remote()).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn forwarded_header_is_not_owner() {
    let app = test_app().await;
    let mut req = request("/api/v1/agents", loopback());
    req.headers_mut()
        .insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
    let res = app.app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bearer_token_with_scope_is_accepted() {
    let (app, token) = test_app_with_device(&["read"]).await;
    assert_eq!(
        get_with_bearer(app.app, "/api/v1/agents", &token)
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn cookie_mutation_without_csrf_is_403() {
    let (app, token, _) = test_app_with_cookie_device().await;
    let res = post_with_cookie(
        app.app.clone(),
        "/api/v1/alerts/a/ack",
        serde_json::json!({}),
        &token,
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let j = body_json(res).await;
    assert_eq!(j["code"], "forbidden");
    assert!(j["request_id"].as_str().is_some());

    let res = post_with_cookie(
        app.app,
        "/api/v1/alerts/a/ack",
        serde_json::json!({}),
        &token,
        Some("csrf-value"),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn ws_ticket_is_single_use_and_expires() {
    let app = test_app().await;
    let ticket = issue_ws_ticket(&app.app).await;
    assert!(consume_ws_ticket(&app, &ticket).await);
    assert!(!consume_ws_ticket(&app, &ticket).await);

    let ticket = issue_ws_ticket(&app.app).await;
    app.state
        .tickets
        .lock()
        .unwrap()
        .get_mut(&ticket)
        .unwrap()
        .expires_at_ms = 1;
    assert!(!consume_ws_ticket(&app, &ticket).await);
}

#[tokio::test]
async fn pairing_code_exchanges_for_scoped_token_once() {
    let app = test_app_with_pairing_code("ABCD-1234", &["read"]).await;
    let res = post(
        app.app.clone(),
        "/api/v1/auth/pair",
        serde_json::json!({"code":"ABCD-1234","name":"hp"}),
        remote(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let token = body_json(res).await["token"].as_str().unwrap().to_string();
    assert_eq!(
        get_with_bearer(app.app.clone(), "/api/v1/health", &token)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post(
            app.app,
            "/api/v1/auth/pair",
            serde_json::json!({"code":"ABCD-1234","name":"hp"}),
            remote(),
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn devices_can_be_listed_and_revoked() {
    let (app, token) = test_app_with_device(&["read"]).await;
    let res = get(app.app.clone(), "/api/v1/devices", loopback()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert_eq!(j[0]["id"], "dev1");
    assert!(j[0].get("token_hash").is_none());

    assert_eq!(
        delete(app.app.clone(), "/api/v1/devices/dev1", loopback())
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        get_with_bearer(app.app, "/api/v1/health", &token)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn loopback_without_trust_requires_token() {
    let store = Store::open(StoreOptions::for_dir(tempfile::tempdir().unwrap().path()))
        .await
        .unwrap();
    let app = hq_api::build_router(AppState {
        store,
        ring: std::sync::Arc::new(std::sync::Mutex::new(PatchRing::new(1000))),
        auth: hq_api::auth::AuthConfig {
            trust_loopback: false,
        },
        started_at: std::time::Instant::now(),
        tickets: Default::default(),
    });
    assert_eq!(
        get(app, "/api/v1/health", loopback()).await.status(),
        StatusCode::UNAUTHORIZED
    );
}
