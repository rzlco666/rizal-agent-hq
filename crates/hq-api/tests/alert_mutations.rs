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

#[tokio::test]
async fn snooze_overflowing_minutes_does_not_panic() {
    let app = seeded_alert_app().await;
    let res = post(
        app.app,
        "/api/v1/alerts/a1/snooze",
        serde_json::json!({"minutes": i64::MAX}),
        loopback(),
    )
    .await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn second_snooze_with_new_deadline_emits_second_event() {
    let app = seeded_alert_app().await;
    let first: serde_json::Value = body_json(
        post(
            app.app.clone(),
            "/api/v1/alerts/a1/snooze",
            serde_json::json!({"minutes": 15}),
            loopback(),
        )
        .await,
    )
    .await;
    let first_until = first["snooze_until_ms"].as_i64().unwrap();
    let events_after_first = app.store.count_events().await.unwrap();

    let second: serde_json::Value = body_json(
        post(
            app.app.clone(),
            "/api/v1/alerts/a1/snooze",
            serde_json::json!({"minutes": 30}),
            loopback(),
        )
        .await,
    )
    .await;
    assert_eq!(second["state"], "snoozed");
    let second_until = second["snooze_until_ms"].as_i64().unwrap();
    assert!(
        second_until > first_until,
        "second snooze must extend deadline: {first} then {second}"
    );
    assert_eq!(
        app.store.count_events().await.unwrap(),
        events_after_first + 1,
        "a changed snooze deadline must emit a new event"
    );

    let page = app
        .store
        .query_events(hq_store::EventQuery {
            kind: Some("alert_snoozed".to_string()),
            subject: Some("a1".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    let deadlines: Vec<i64> = page
        .events
        .iter()
        .filter_map(|event| event.draft.payload["snooze_until_ms"].as_i64())
        .collect();
    assert_eq!(
        deadlines.len(),
        2,
        "expected two snooze events: {deadlines:?}"
    );
    assert_ne!(deadlines[0], deadlines[1]);
}
