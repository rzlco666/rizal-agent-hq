#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{
        HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode, header,
    },
    response::Response,
};
use hq_api::{
    AppState,
    auth::{AuthConfig, token_hash},
    build_router,
};
use hq_store::*;
use std::net::SocketAddr;
use tempfile::TempDir;
use tower::ServiceExt;

pub struct TestApp {
    pub app: Router,
    pub addr: SocketAddr,
    pub state: AppState,
    pub store: Store,
    pub ingestor: Ingestor,
    pub _dir: TempDir,
}

pub fn state_for(store: Store) -> (AppState, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (
        AppState {
            store,
            ring: std::sync::Arc::new(std::sync::Mutex::new(PatchRing::new(1000))),
            auth: AuthConfig {
                trust_loopback: true,
            },
            started_at: std::time::Instant::now(),
            tickets: Default::default(),
        },
        dir,
    )
}

pub async fn test_app() -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let ingestor = Ingestor::new(store.clone());
    let state = AppState {
        store: store.clone(),
        ring: std::sync::Arc::new(std::sync::Mutex::new(PatchRing::new(1000))),
        auth: AuthConfig {
            trust_loopback: true,
        },
        started_at: std::time::Instant::now(),
        tickets: Default::default(),
    };
    let app = build_router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = app.clone();
    tokio::spawn(async move {
        axum::serve(
            listener,
            server.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    TestApp {
        app,
        addr,
        state,
        store,
        ingestor,
        _dir: dir,
    }
}

pub async fn test_app_with_device(scopes: &[&str]) -> (TestApp, String) {
    let app = test_app().await;
    let token = "tok_test";
    app.store
        .insert_device(DeviceRecord {
            id: "dev1".to_string(),
            name: "hp".to_string(),
            token_hash: token_hash(token),
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
            created_ms: 1_700_000_000_000,
            expires_ms: None,
            last_seen_ms: None,
            revoked_ms: None,
        })
        .await
        .unwrap();
    (app, token.to_string())
}

pub async fn test_app_with_cookie_device() -> (TestApp, String, String) {
    let (app, token) = test_app_with_device(&["read", "write"]).await;
    (app, token, "csrf-value".to_string())
}

pub async fn test_app_with_pairing_code(code: &str, scopes: &[&str]) -> TestApp {
    let app = test_app().await;
    app.store
        .create_pairing(
            &token_hash(code),
            PairingRecord {
                name: "hp".to_string(),
                scopes: scopes.iter().map(|s| s.to_string()).collect(),
                created_ms: hq_api::auth::now_ms(),
                expires_ms: hq_api::auth::now_ms() + 600_000,
            },
        )
        .await
        .unwrap();
    app
}

pub fn loopback() -> SocketAddr {
    "127.0.0.1:45671".parse().unwrap()
}

pub fn remote() -> SocketAddr {
    "203.0.113.9:45671".parse().unwrap()
}

pub fn request(uri: &str, peer: SocketAddr) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .extension(ConnectInfo(peer))
        .body(Body::empty())
        .unwrap()
}

pub async fn get(app: Router, uri: &str, peer: SocketAddr) -> Response {
    app.oneshot(request(uri, peer)).await.unwrap()
}

pub async fn get_with_bearer(app: Router, uri: &str, token: &str) -> Response {
    let mut req = request(uri, remote());
    req.headers_mut().insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    app.oneshot(req).await.unwrap()
}

pub async fn post(
    app: Router,
    uri: &str,
    body: serde_json::Value,
    peer: SocketAddr,
) -> Response {
    let mut req = Request::builder()
        .uri(uri)
        .method(Method::POST)
        .extension(ConnectInfo(peer))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    req.headers_mut().extend(ws_upgrade_headers());
    app.oneshot(req).await.unwrap()
}

pub async fn post_with_cookie(
    app: Router,
    uri: &str,
    body: serde_json::Value,
    token: &str,
    csrf: Option<&str>,
) -> Response {
    let mut req = Request::builder()
        .uri(uri)
        .method(Method::POST)
        .extension(ConnectInfo(remote()))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("hq_session={token}; csrf=csrf-value"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    if let Some(csrf) = csrf {
        req.headers_mut().insert(
            HeaderName::from_static("x-csrf-token"),
            HeaderValue::from_str(csrf).unwrap(),
        );
    }
    req.headers_mut().extend(ws_upgrade_headers());
    app.oneshot(req).await.unwrap()
}

pub async fn delete(app: Router, uri: &str, peer: SocketAddr) -> Response {
    let mut req = request(uri, peer);
    *req.method_mut() = Method::DELETE;
    app.oneshot(req).await.unwrap()
}

pub async fn body_json(res: Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

pub async fn body_text(res: Response) -> String {
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn ws_upgrade_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
    headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
    headers.insert(
        header::SEC_WEBSOCKET_KEY,
        HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="),
    );
    headers.insert(
        header::SEC_WEBSOCKET_VERSION,
        HeaderValue::from_static("13"),
    );
    headers
}

pub async fn issue_ws_ticket(app: &Router) -> String {
    let res = post(app.clone(), "/api/v1/auth/ws-ticket", serde_json::json!({}), loopback()).await;
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["ticket"]
        .as_str()
        .unwrap()
        .to_string()
}

pub async fn consume_ws_ticket(app: &TestApp, ticket: &str) -> bool {
    match tokio_tungstenite::connect_async(format!("ws://{}/ws?ticket={ticket}", app.addr)).await {
        Ok((_socket, response)) => response.status() == StatusCode::SWITCHING_PROTOCOLS,
        Err(error) => {
            println!("ws ticket rejected: {error}");
            false
        }
    }
}

pub fn draft(kind: EventKind, subject: Subject, payload: serde_json::Value) -> EventDraft {
    EventDraft {
        event_ts_ms: 1_700_000_000_000,
        kind,
        subject,
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload,
    }
}

pub fn agent_observed(id: &str, status: &str) -> EventDraft {
    draft(
        EventKind::AgentObserved,
        Subject::Agent(AgentId(id.to_string())),
        serde_json::json!({
            "name": id,
            "role": "agent",
            "kind": "utama",
            "runtime_key": id,
            "status": status,
            "evidence": "L1"
        }),
    )
}

pub fn alert_raised(id: &str) -> EventDraft {
    draft(
        EventKind::AlertRaised,
        Subject::Alert(AlertId(id.to_string())),
        serde_json::json!({
            "id": id,
            "fingerprint": format!("fp-{id}"),
            "rule_id": "rule-1",
            "level": "warning",
            "message": "perlu perhatian"
        }),
    )
}
