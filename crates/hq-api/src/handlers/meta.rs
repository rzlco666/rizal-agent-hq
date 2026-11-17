use crate::{error::ApiError, state::AppState};
use axum::{
    Json,
    extract::{ConnectInfo, Request, State},
};
use hq_core::{EventDraft, EventId, EventKind, SourceId, Subject};
use serde::Deserialize;
use serde_json::Value;
use std::net::SocketAddr;
use utoipa::OpenApi;

pub const SEED_ENV: &str = "HQ_ALLOW_SEED";

#[derive(Debug, Deserialize)]
struct SeedDraft {
    event_ts_ms: i64,
    kind: EventKind,
    subject: Subject,
    source: SourceId,
    #[serde(default)]
    correlation_id: Option<String>,
    #[serde(default)]
    causation_id: Option<String>,
    #[serde(default)]
    idempotency_key: Option<String>,
    payload: Value,
}

fn parse_event_id(value: &str) -> Result<EventId, String> {
    ulid::Ulid::from_string(value)
        .map(EventId)
        .map_err(|error| format!("invalid event id {value}: {error}"))
}

/// Parse either `{events: [...]}` or a bare array into validated drafts.
pub fn parse_seed_events(value: &Value) -> Result<Vec<EventDraft>, String> {
    let items = match value {
        Value::Array(items) => items,
        Value::Object(object) => object
            .get("events")
            .and_then(Value::as_array)
            .ok_or_else(|| "expected an events array".to_string())?,
        _ => return Err("expected an events array".to_string()),
    };
    items
        .iter()
        .map(|item| {
            let draft: SeedDraft =
                serde_json::from_value(item.clone()).map_err(|e| format!("invalid event: {e}"))?;
            Ok(EventDraft {
                event_ts_ms: draft.event_ts_ms,
                kind: draft.kind,
                subject: draft.subject,
                source: draft.source,
                correlation_id: draft.correlation_id.as_deref().map(parse_event_id).transpose()?,
                causation_id: draft.causation_id.as_deref().map(parse_event_id).transpose()?,
                idempotency_key: draft.idempotency_key,
                payload: draft.payload,
            })
        })
        .collect()
}

/// Checked on every request: an unset/false gate hides the route entirely;
/// an enabled gate is reachable only from loopback.
pub fn seed_gate(peer: Option<SocketAddr>) -> Result<(), ApiError> {
    if std::env::var(SEED_ENV).ok().as_deref() != Some("1") {
        return Err(ApiError::NotFound);
    }
    if !peer.is_some_and(|peer| peer.ip().is_loopback()) {
        return Err(ApiError::Forbidden("loopback peer required".to_string()));
    }
    Ok(())
}

pub async fn dev_events(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    seed_gate(Some(peer))?;
    let bytes = axum::body::to_bytes(request.into_body(), 4 * 1024 * 1024)
        .await
        .map_err(|e| ApiError::BadRequest(format!("cannot read body: {e}")))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON: {e}")))?;
    let drafts = parse_seed_events(&value).map_err(ApiError::BadRequest)?;
    let report = hq_store::Ingestor::new(state.store.clone()).ingest(drafts).await?;
    let seq = state.store.last_seq().await?.0;
    Ok(Json(serde_json::json!({
        "accepted": report.accepted.len(),
        "duplicates": report.duplicates,
        "seq": seq,
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/openapi.json",
    tag = "Meta",
    responses((status = 200, description = "OpenAPI document", body = serde_json::Value))
)]
pub async fn openapi_json(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let doc = crate::openapi::ApiDoc::openapi();
    Ok(Json(
        serde_json::to_value(doc).map_err(|e| ApiError::Internal(e.to_string()))?,
    ))
}
