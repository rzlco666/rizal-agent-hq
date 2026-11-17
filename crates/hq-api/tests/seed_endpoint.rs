mod common;

use crate::common::*;
use axum::http::StatusCode;
use std::sync::LazyLock;
use tokio::sync::Mutex;

static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct SeedEnv(Option<std::ffi::OsString>);

impl SeedEnv {
    fn set(value: Option<&str>) -> Self {
        let previous = std::env::var_os("HQ_ALLOW_SEED");
        // SAFETY: tests take ENV_LOCK; all env access in this binary is serialized by it.
        unsafe {
            match value {
                Some(value) => std::env::set_var("HQ_ALLOW_SEED", value),
                None => std::env::remove_var("HQ_ALLOW_SEED"),
            }
        }
        Self(previous)
    }
}

impl Drop for SeedEnv {
    fn drop(&mut self) {
        // SAFETY: the guard owning this value is held until after restoration.
        unsafe {
            match &self.0 {
                Some(value) => std::env::set_var("HQ_ALLOW_SEED", value),
                None => std::env::remove_var("HQ_ALLOW_SEED"),
            }
        }
    }
}

fn seed_event(ts: i64) -> serde_json::Value {
    serde_json::json!({
        "event_ts_ms": ts,
        "kind": "agent_observed",
        "subject": "agent:omp",
        "source": "test:e2e",
        "idempotency_key": "seed-e2e-agent-omp",
        "payload": {
            "observed_at_ms": ts,
            "runtime_key": "omp",
            "status": "bekerja",
            "kind": "Utama",
            "activity": {"key": "agent.process_running", "params": {"family": "omp"}},
            "processes": 1,
            "cpu_percent": 0.0,
            "evidence": "L1"
        }
    })
}

fn wrapped_body() -> serde_json::Value {
    serde_json::json!({ "events": [seed_event(1_700_000_000_000_i64)] })
}

#[tokio::test]
async fn dev_events_returns_404_for_loopback_when_env_gate_is_off() {
    let _lock = ENV_LOCK.lock().await;
    let _env = SeedEnv::set(None);
    let app = test_app().await;

    let res = post(
        app.app.clone(),
        "/api/v1/dev/events",
        wrapped_body(),
        loopback(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let overview = body_json(get(app.app, "/api/v1/overview", loopback()).await).await;
    assert_eq!(overview["seq"], 0);
}

#[tokio::test]
async fn dev_events_returns_403_for_remote_peer_when_env_gate_is_on() {
    let _lock = ENV_LOCK.lock().await;
    let _env = SeedEnv::set(Some("1"));
    let app = test_app().await;

    let res = post(
        app.app.clone(),
        "/api/v1/dev/events",
        wrapped_body(),
        remote(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let overview = body_json(get(app.app, "/api/v1/overview", loopback()).await).await;
    assert_eq!(overview["seq"], 0);
}

#[tokio::test]
async fn dev_events_ingests_loopback_events_visible_in_overview() {
    let _lock = ENV_LOCK.lock().await;
    let _env = SeedEnv::set(Some("1"));
    let app = test_app().await;

    let res = post(
        app.app.clone(),
        "/api/v1/dev/events",
        wrapped_body(),
        loopback(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let report = body_json(res).await;
    assert_eq!(report["accepted"], 1);
    assert_eq!(report["duplicates"], 0);
    assert!(report["seq"].as_i64().is_some_and(|seq| seq > 0));

    let overview = body_json(get(app.app, "/api/v1/overview", loopback()).await).await;
    assert!(
        overview["agents"]
            .as_array()
            .is_some_and(|agents| agents.iter().any(|agent| agent["id"] == "agent:omp"))
    );
    assert!(
        overview["counts"]["working"]
            .as_i64()
            .is_some_and(|count| count >= 1)
    );
}

#[tokio::test]
async fn dev_events_accepts_bare_array_and_reports_duplicates() {
    let _lock = ENV_LOCK.lock().await;
    let _env = SeedEnv::set(Some("1"));
    let app = test_app().await;
    let body = serde_json::json!([seed_event(1_700_000_000_000_i64)]);

    let first = body_json(
        post(
            app.app.clone(),
            "/api/v1/dev/events",
            body.clone(),
            loopback(),
        )
        .await,
    )
    .await;
    assert_eq!(first["accepted"], 1);
    let second = body_json(post(app.app, "/api/v1/dev/events", body, loopback()).await).await;
    assert_eq!(second["accepted"], 0);
    assert_eq!(second["duplicates"], 1);
}

#[tokio::test]
async fn dev_events_gate_is_checked_on_each_request_not_cached_at_startup() {
    let _lock = ENV_LOCK.lock().await;
    let app = test_app().await;
    {
        let _env = SeedEnv::set(None);
        let res = post(
            app.app.clone(),
            "/api/v1/dev/events",
            wrapped_body(),
            loopback(),
        )
        .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
    {
        let _env = SeedEnv::set(Some("1"));
        let res = post(app.app, "/api/v1/dev/events", wrapped_body(), loopback()).await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[test]
fn openapi_document_does_not_expose_dev_events_route() {
    let doc =
        serde_json::to_value(<hq_api::openapi::ApiDoc as utoipa::OpenApi>::openapi()).unwrap();
    assert!(
        !doc["paths"]
            .as_object()
            .unwrap()
            .contains_key("/api/v1/dev/events")
    );
}
