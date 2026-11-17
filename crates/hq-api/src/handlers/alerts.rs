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
