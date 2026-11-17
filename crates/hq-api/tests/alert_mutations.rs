mod common;

use crate::common::*;

#[tokio::test]
async fn ack_alert_updates_state_and_emits_event() {
    let app = seeded_alert_app().await;
    let res = post(
        app.app.clone(),
        "/api/v1/alerts/a1/ack",
        serde_json::json!({"note":"sudah dicek"}),
        loopback(),
    )
    .await;
    assert_eq!(res.status(), 200);
    let j = body_json(res).await;
    assert_eq!(j["state"], "acknowledged");
    assert!(j["seq"].as_i64().unwrap() > 0);

    let before = app.store.count_events().await.unwrap();
    let res = post(
        app.app.clone(),
        "/api/v1/alerts/a1/ack",
        serde_json::json!({"note":"lagi dicek"}),
        loopback(),
    )
    .await;
    assert_eq!(res.status(), 200);
    assert_eq!(body_json(res).await["state"], "acknowledged");
    assert_eq!(app.store.count_events().await.unwrap(), before);
}

#[tokio::test]
async fn snooze_and_resolve_move_alert_state() {
    let app = seeded_alert_app().await;
    let res = post(
        app.app.clone(),
        "/api/v1/alerts/a1/snooze",
        serde_json::json!({"minutes": 15}),
        loopback(),
    )
    .await;
    assert_eq!(res.status(), 200);
    let j = body_json(res).await;
    assert_eq!(j["state"], "snoozed");
    assert!(j["snooze_until_ms"].as_i64().unwrap() > 0);

    let res = post(
        app.app,
        "/api/v1/alerts/a1/resolve",
        serde_json::json!({}),
        loopback(),
    )
    .await;
    assert_eq!(res.status(), 200);
    assert_eq!(body_json(res).await["state"], "resolved");
}
