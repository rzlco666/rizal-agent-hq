use crate::{dto, error::ApiError, state::AppState};
use axum::{
    Json,
    extract::{Query, State},
};
use hq_store::UsageQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct UsageRequestQuery {
    pub agent: Option<String>,
    pub group_by: Option<String>,
    pub window: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/usage",
    tag = "Usage",
    params(UsageRequestQuery),
    responses((status = 200, description = "Usage totals", body = dto::UsageReportDto))
)]
pub async fn usage(
    State(state): State<AppState>,
    Query(query): Query<UsageRequestQuery>,
) -> Result<Json<dto::UsageReportDto>, ApiError> {
    let report = state
        .store
        .usage(UsageQuery {
            agent: query.agent,
            kind: "minutes".to_string(),
        })
        .await?;
    Ok(Json(dto::UsageReportDto {
        total: report.total.into(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/costs",
    tag = "Usage",
    params(UsageRequestQuery),
    responses((status = 200, description = "Cost totals", body = dto::CostReport))
)]
pub async fn costs(State(state): State<AppState>) -> Result<Json<dto::CostReport>, ApiError> {
    Ok(Json(state.store.costs().await?.into()))
}
