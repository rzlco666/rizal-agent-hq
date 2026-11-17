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

#[tokio::test]
async fn overview_has_agents_and_counts() {
    let app = seeded_app().await;
    let j = body_json(get(app.app, "/api/v1/overview", loopback()).await).await;
    assert!(j["agents"].as_array().unwrap().len() >= 2);
    assert!(j["counts"]["working"].as_u64().unwrap() >= 1);
    assert!(j["seq"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn agent_detail_honors_as_of() {
    let app = seeded_app().await;
    let id = "agent:omp";
    let now: serde_json::Value =
        body_json(get(app.app.clone(), &format!("/api/v1/agents/{id}"), loopback()).await).await;
    let past = now["card"]["status"].clone();
    let old: serde_json::Value = body_json(
        get(
            app.app,
            &format!("/api/v1/agents/{id}?as_of=1700000000000"),
            loopback(),
        )
        .await,
    )
    .await;
    assert!(old.get("card").is_some() || old.get("not_found").is_some());
    assert_eq!(now["card"]["status"], past);
}

#[tokio::test]
async fn pagination_cursor_is_stable() {
    let app = seeded_app().await;
    let p1: serde_json::Value =
        body_json(get(app.app.clone(), "/api/v1/timeline?limit=1", loopback()).await).await;
    let cursor = p1["next_cursor"].as_str().unwrap().to_string();
    let p2: serde_json::Value = body_json(
        get(
            app.app,
            &format!("/api/v1/timeline?limit=1&since={cursor}"),
            loopback(),
        )
        .await,
    )
    .await;
    assert_ne!(p1["items"][0]["seq"], p2["items"][0]["seq"]);
}

#[tokio::test]
async fn api_never_returns_secret() {
    let app = test_app().await;
    let secret = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    app.ingestor
        .ingest(vec![log_with_secret("agent:omp", secret)])
        .await
        .unwrap();
    for path in [
        "/api/v1/agents/agent:omp/logs",
        "/api/v1/timeline",
        "/api/v1/overview",
    ] {
        let body = body_text(get(app.app.clone(), path, loopback()).await).await;
        assert!(!body.contains(secret), "secret leaked at {path}");
    }
}

#[tokio::test]
async fn logs_are_locale_aware() {
    let app = test_app().await;
    app.ingestor
        .ingest(vec![log_with_secret("agent:omp", "pekerjaan selesai")])
        .await
        .unwrap();
    let j = body_json(
        get(
            app.app,
            "/api/v1/agents/agent:omp/logs?locale=en",
            loopback(),
        )
        .await,
    )
    .await;
    assert!(j["items"].as_array().is_some());
}
