use crate::{dto, error::ApiError, state::AppState};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use hq_core::AgentId;
use hq_store::Locale;
use hq_store::{UsageQuery, UsageTotals};
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListQuery {
    pub status: Option<String>,
    pub kind: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct DetailQuery {
    pub as_of: Option<i64>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LogQuery {
    pub level: Option<String>,
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub before: Option<i64>,
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LimitQuery {
    pub limit: Option<usize>,
}

fn locale_of(value: Option<&str>) -> Locale {
    match value.unwrap_or("id") {
        "en" => Locale::En,
        "zh" => Locale::Zh,
        _ => Locale::Id,
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/agents",
    tag = "Agents",
    params(ListQuery),
    responses((status = 200, description = "Agent cards", body = dto::AgentPage))
)]
pub async fn agents(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<dto::AgentPage>, ApiError> {
    let mut cards: Vec<dto::AgentCardDto> = state
        .store
        .agents()
        .await?
        .into_iter()
        .map(|detail| detail.card.into())
        .collect();
    if let Some(status) = query.status {
        cards.retain(|card| card.status == status);
    }
    if let Some(q) = query.q {
        cards.retain(|card| card.name.contains(&q) || card.id.contains(&q));
    }
    let _ = query.kind;
    Ok(Json(dto::AgentPage {
        items: cards,
        next_cursor: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}",
    tag = "Agents",
    params(("id" = String, Path, description = "Stable agent id")),
    responses((status = 200, description = "Agent detail", body = dto::AgentDetailDto))
)]
pub async fn agent_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DetailQuery>,
) -> Result<Json<dto::AgentDetailDto>, ApiError> {
    let agent_id = AgentId(id);
    let detail = state
        .store
        .agent(&agent_id, query.as_of)
        .await?
        .ok_or(ApiError::NotFound)?;
    let mut result = dto::AgentDetailDto::from(detail);
    result.as_of = query.as_of;
    result.tasks = state
        .store
        .tasks(Some(&agent_id.0), None, None, Locale::Id, 20)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    result.sessions = state
        .store
        .sessions(&agent_id.0, 20)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}/logs",
    tag = "Agents",
    params(("id" = String, Path, description = "Stable agent id"), LogQuery),
    responses((status = 200, description = "Rendered log page", body = dto::LogPage))
)]
pub async fn agent_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Result<Json<dto::LogPage>, ApiError> {
    let limit = query.limit.unwrap_or(50).min(500);
    let locale = locale_of(query.locale.as_deref());
    let mut items: Vec<dto::LogView> = state
        .store
        .logs(&AgentId(id), locale, limit)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    if let Some(level) = query.level {
        items.retain(|entry| entry.level == level);
    }
    if let Some(q) = query.q {
        items.retain(|entry| entry.text.contains(&q));
    }
    if let Some(before) = query.before {
        items.retain(|entry| entry.ts_ms < before);
    }
    Ok(Json(dto::LogPage {
        items,
        next_cursor: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}/tasks",
    tag = "Agents",
    params(("id" = String, Path, description = "Stable agent id"), LimitQuery),
    responses((status = 200, description = "Agent tasks", body = dto::TaskPage))
)]
pub async fn agent_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<dto::TaskPage>, ApiError> {
    let items = state
        .store
        .tasks(Some(&id), None, None, Locale::Id, query.limit.unwrap_or(50))
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(dto::TaskPage {
        items,
        next_cursor: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}/sessions",
    tag = "Agents",
    params(("id" = String, Path, description = "Stable agent id"), LimitQuery),
    responses((status = 200, description = "Agent sessions", body = dto::SessionPage))
)]
pub async fn agent_sessions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<dto::SessionPage>, ApiError> {
    let items = state
        .store
        .sessions(&id, query.limit.unwrap_or(50))
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(dto::SessionPage {
        items,
        next_cursor: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}/files",
    tag = "Agents",
    params(("id" = String, Path, description = "Stable agent id")),
    responses((status = 200, description = "Changed files", body = [dto::ChangedFile]))
)]
pub async fn agent_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<dto::ChangedFile>>, ApiError> {
    let files = state
        .store
        .files(&id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(files))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}/usage",
    tag = "Agents",
    params(("id" = String, Path, description = "Stable agent id")),
    responses((status = 200, description = "Agent usage", body = dto::UsageReportDto))
)]
pub async fn agent_usage(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<dto::UsageReportDto>, ApiError> {
    let report = state.store.usage(UsageQuery::agent_minutes(&id)).await?;
    let totals: UsageTotals = report.total;
    Ok(Json(dto::UsageReportDto {
        total: totals.into(),
    }))
}
