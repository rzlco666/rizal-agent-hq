use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hq_core::{Actor, DeviceId, EventDraft, EventKind, SourceId, Subject};
use hq_store::DeviceRecord;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CSRF_HEADER: &str = "x-csrf-token";
pub const CSRF_COOKIE: &str = "csrf";
pub const SESSION_COOKIE: &str = "hq_session";

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub trust_loopback: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            trust_loopback: std::env::var_os("HQ_TRUST_LOOPBACK").is_none_or(|v| v != "0"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub actor: Actor,
    pub device_id: Option<DeviceId>,
    pub scopes: Vec<String>,
    pub via_cookie: bool,
}

impl AuthContext {
    pub fn owner() -> Self {
        Self {
            actor: Actor::Owner,
            device_id: None,
            scopes: vec!["*".to_string()],
            via_cookie: false,
        }
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == "*" || s == scope)
    }
}

#[derive(Debug, Clone)]
pub struct WsTicket {
    pub device_id: Option<DeviceId>,
    pub expires_at_ms: i64,
}

pub type TicketStore = Arc<Mutex<HashMap<String, WsTicket>>>;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PairResponse {
    pub token: String,
    pub device_id: String,
    pub expires_ms: Option<i64>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WsTicketResponse {
    pub ticket: String,
    pub expires_ms: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_ms: i64,
    pub expires_ms: Option<i64>,
    pub revoked_ms: Option<i64>,
}

impl From<DeviceRecord> for DeviceView {
    fn from(device: DeviceRecord) -> Self {
        Self {
            id: device.id,
            name: device.name,
            scopes: device.scopes,
            created_ms: device.created_ms,
            expires_ms: device.expires_ms,
            revoked_ms: device.revoked_ms,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PairInput {
    pub code: String,
    pub name: Option<String>,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn token_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn random_token() -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// Uppercase alphanumeric pairing code; `hq device add` prints it once.
pub fn random_code(len: usize) -> Result<String, ApiError> {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(bytes
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect())
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.trim() == name)
        .map(|(_, value)| value.trim().to_string())
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    raw.strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

pub async fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<AuthContext, ApiError> {
    if let Some(token) = bearer_token(headers).or_else(|| cookie_value(headers, SESSION_COOKIE)) {
        let device = state
            .store
            .device_by_token_hash(&token_hash(&token))
            .await?
            .ok_or(ApiError::Unauthorized)?;
        if device.revoked_ms.is_some()
            || device.expires_ms.is_some_and(|expires| expires <= now_ms())
        {
            return Err(ApiError::Unauthorized);
        }
        let via_cookie = bearer_token(headers).is_none();
        return Ok(AuthContext {
            actor: Actor::Automation(format!("device:{}", device.id)),
            device_id: Some(DeviceId(device.id)),
            scopes: device.scopes,
            via_cookie,
        });
    }
    Err(ApiError::Unauthorized)
}

fn is_mutation(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn required_scope(path: &str, method: &Method) -> &'static str {
    if method == Method::DELETE && path.starts_with("/api/v1/devices") {
        "device.manage"
    } else if is_mutation(method) {
        "write"
    } else {
        "read"
    }
}

fn is_public(path: &str) -> bool {
    matches!(
        path,
        "/api/v1/auth/pair" | "/api/v1/openapi.json" | "/openapi.json"
    ) || crate::static_assets::is_public_asset_path(path)
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if is_public(&path) || path == "/ws" || path == "/api/v1/events/stream" {
        return next.run(request).await;
    }
    let Some(ConnectInfo(peer)) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .cloned()
    else {
        return ApiError::Unauthorized.into_response();
    };
    if state.auth.trust_loopback && request.headers().contains_key("x-forwarded-for") {
        return ApiError::Unauthorized.into_response();
    }

    let owner = state.auth.trust_loopback && peer.ip().is_loopback();
    let context = if owner {
        AuthContext::owner()
    } else {
        match authenticate(&state, request.headers()).await {
            Ok(context) => context,
            Err(error) => return error.into_response(),
        }
    };

    let method = request.method().clone();
    if context.via_cookie && is_mutation(&method) {
        let supplied = request
            .headers()
            .get(CSRF_HEADER)
            .and_then(|v| v.to_str().ok());
        let expected = cookie_value(request.headers(), CSRF_COOKIE);
        if expected.is_none() || supplied != expected.as_deref() {
            return ApiError::Forbidden("csrf token mismatch".to_string()).into_response();
        }
    }
    if !context.has_scope(required_scope(&path, &method)) {
        return ApiError::Forbidden("scope required".to_string()).into_response();
    }

    let mut request = request;
    request.extensions_mut().insert(context);
    next.run(request).await
}

#[utoipa::path(post, path = "/api/v1/auth/pair", tag = "Auth", request_body = PairInput, responses((status = 200, description = "Scoped device token", body = PairResponse)))]
pub async fn pair(
    State(state): State<AppState>,
    Json(input): Json<PairInput>,
) -> Result<Json<PairResponse>, ApiError> {
    let code = input.code.trim();
    if code.is_empty() {
        return Err(ApiError::BadRequest("pairing code required".to_string()));
    }
    let record = state
        .store
        .consume_pairing(&token_hash(code), now_ms())
        .await?
        .ok_or_else(|| ApiError::Conflict("pairing code used or expired".to_string()))?;
    let token = random_token()?;
    let device_id = format!("dev-{}", &random_token()?[..12]);
    let name = input.name.unwrap_or(record.name);
    state
        .store
        .insert_device(DeviceRecord {
            id: device_id.clone(),
            name,
            token_hash: token_hash(&token),
            scopes: record.scopes,
            created_ms: now_ms(),
            expires_ms: None,
            last_seen_ms: None,
            revoked_ms: None,
        })
        .await?;
    emit_device_event(&state, EventKind::DeviceAdded, &device_id).await;
    Ok(Json(PairResponse {
        token,
        device_id,
        expires_ms: None,
    }))
}

#[utoipa::path(post, path = "/api/v1/auth/ws-ticket", tag = "Auth", responses((status = 200, description = "Single-use WS ticket", body = WsTicketResponse)))]
pub async fn ws_ticket(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<WsTicketResponse>, ApiError> {
    let context = request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;
    let ticket = random_token()?;
    let expires_ms = now_ms() + 60_000;
    state
        .tickets
        .lock()
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .insert(
            ticket.clone(),
            WsTicket {
                device_id: context.device_id.clone(),
                expires_at_ms: expires_ms,
            },
        );
    Ok(Json(WsTicketResponse { ticket, expires_ms }))
}

/// Single-use WS ticket check. Bound to the requesting device when present.
pub fn consume_ws_ticket(ticket: &str, device_id: Option<&DeviceId>, state: &AppState) -> bool {
    let Ok(mut tickets) = state.tickets.lock() else {
        return false;
    };
    let Some(found) = tickets.get(ticket) else {
        return false;
    };
    if found.expires_at_ms <= now_ms()
        || (found.device_id.is_some() && found.device_id != device_id.cloned())
    {
        tickets.remove(ticket);
        return false;
    }
    tickets.remove(ticket);
    true
}

#[utoipa::path(get, path = "/api/v1/devices", tag = "Auth", responses((status = 200, description = "Devices", body = [DeviceView])))]
pub async fn devices(State(state): State<AppState>) -> Result<Json<Vec<DeviceView>>, ApiError> {
    let records = state.store.devices().await?;
    Ok(Json(records.into_iter().map(DeviceView::from).collect()))
}

#[utoipa::path(delete, path = "/api/v1/devices/{id}", tag = "Auth", responses((status = 204, description = "Device revoked")))]
pub async fn revoke_device(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    state.store.revoke_device(&id, now_ms()).await?;
    emit_device_event(&state, EventKind::DeviceRevoked, &id).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn emit_device_event(state: &AppState, kind: EventKind, device_id: &str) {
    let draft = EventDraft {
        event_ts_ms: now_ms(),
        kind,
        subject: Subject::Device(DeviceId(device_id.to_string())),
        source: SourceId("user".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload: serde_json::json!({"device_id": device_id}),
    };
    if let Err(error) = state.store.append(vec![draft]).await {
        tracing::warn!(%error, "device event append failed");
    }
}
