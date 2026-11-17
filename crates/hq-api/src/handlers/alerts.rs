use crate::{dto, error::ApiError, state::AppState};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AlertQuery {
    pub state: Option<String>,
    pub level: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/alerts",
    tag = "Alerts",
    params(AlertQuery),
    responses((status = 200, description = "Alerts", body = [dto::AlertView]))
)]
pub async fn alerts(
    State(state): State<AppState>,
    Query(query): Query<AlertQuery>,
) -> Result<Json<Vec<dto::AlertView>>, ApiError> {
    let items = state
        .store
        .alerts(query.state.as_deref(), query.level.as_deref())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/api/v1/alerts/{id}",
    tag = "Alerts",
    params(("id" = String, Path, description = "Alert id")),
    responses((status = 200, description = "Alert detail", body = dto::AlertView))
)]
pub async fn alert_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<dto::AlertView>, ApiError> {
    let alert = state.store.alert(&id).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(alert.into()))
}

#[derive(Debug, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct AlertMutation {
    pub note: Option<String>,
    pub minutes: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/api/v1/alerts/{id}/ack",
    tag = "Alerts",
    request_body = AlertMutation,
    responses((status = 200, description = "Alert acknowledged", body = dto::AlertActionResult))
)]
pub async fn ack_alert(
    state: axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<AlertMutation>,
) -> Result<axum::Json<dto::AlertActionResult>, ApiError> {
    mutate(
        state,
        id,
        hq_core::EventKind::AlertAcknowledged,
        "acknowledged",
        body,
        "alert_acknowledged",
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/alerts/{id}/snooze",
    tag = "Alerts",
    request_body = AlertMutation,
    responses((status = 200, description = "Alert snoozed", body = dto::AlertActionResult))
)]
pub async fn snooze_alert(
    state: axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<AlertMutation>,
) -> Result<axum::Json<dto::AlertActionResult>, ApiError> {
    mutate(
        state,
        id,
        hq_core::EventKind::AlertSnoozed,
        "snoozed",
        body,
        "alert_snoozed",
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/alerts/{id}/resolve",
    tag = "Alerts",
    request_body = AlertMutation,
    responses((status = 200, description = "Alert resolved", body = dto::AlertActionResult))
)]
pub async fn resolve_alert(
    state: axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<AlertMutation>,
) -> Result<axum::Json<dto::AlertActionResult>, ApiError> {
    mutate(
        state,
        id,
        hq_core::EventKind::AlertResolved,
        "resolved",
        body,
        "alert_resolved",
    )
    .await
}

async fn mutate(
    state: axum::extract::State<AppState>,
    id: String,
    kind: hq_core::EventKind,
    target_state: &str,
    body: AlertMutation,
    kind_name: &str,
) -> Result<axum::Json<dto::AlertActionResult>, ApiError> {
    let current = state.store.alert(&id).await?.ok_or(ApiError::NotFound)?;
    let now = crate::auth::now_ms();
    let mut snooze_until_ms = current.snooze_until_ms;
    let seq = if current.state == target_state {
        last_event_seq(&state, &id, kind_name).await?.unwrap_or(0)
    } else {
        let until_ms = now + body.minutes.unwrap_or(15).max(0) * 60_000;
        if kind == hq_core::EventKind::AlertSnoozed {
            snooze_until_ms = Some(until_ms);
        }
        let mut payload = serde_json::json!({
            "id": id,
            "actor": "owner",
            "at_ms": now,
            "until_ms": until_ms,
            "snooze_until_ms": until_ms,
        });
        if let Some(note) = body.note {
            payload["note"] = serde_json::json!(note);
        }
        let draft = hq_core::EventDraft {
            event_ts_ms: now,
            kind,
            subject: hq_core::Subject::Alert(hq_core::AlertId(id.clone())),
            source: hq_core::SourceId("user".to_string()),
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(format!("api-alert-{id}-{kind_name}-{now}")),
            payload,
        };
        let ingestor = hq_store::Ingestor::new(state.store.clone());
        ingestor
            .ingest(vec![draft])
            .await?
            .accepted
            .first()
            .map(|event| event.seq.0)
            .unwrap_or(0)
    };
    Ok(axum::Json(dto::AlertActionResult {
        id,
        state: target_state.to_string(),
        seq,
        snooze_until_ms,
    }))
}

async fn last_event_seq(state: &AppState, id: &str, kind: &str) -> Result<Option<i64>, ApiError> {
    let page = state
        .store
        .query_events(hq_store::EventQuery {
            kind: Some(kind.to_string()),
            subject: Some(format!("alert:{id}")),
            limit: Some(1000),
            ..Default::default()
        })
        .await?;
    Ok(page.events.iter().map(|event| event.seq.0).max())
}
