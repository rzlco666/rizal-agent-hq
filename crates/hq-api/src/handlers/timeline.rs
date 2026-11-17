use crate::{dto, error::ApiError, state::AppState};
use axum::{
    Json,
    extract::{Query, State},
};
use hq_store::EventQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TimelineQuery {
    /// Seq cursor. Both `since` and `cursor` are accepted.
    pub since: Option<i64>,
    pub cursor: Option<i64>,
    pub until: Option<i64>,
    pub kind: Option<String>,
    pub subject: Option<String>,
    pub limit: Option<usize>,
}

#[utoipa::path(
    get,
    path = "/api/v1/timeline",
    tag = "Timeline",
    params(TimelineQuery),
    responses((status = 200, description = "Event page", body = dto::EventPage))
)]
pub async fn timeline(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<dto::EventPage>, ApiError> {
    let page = state
        .store
        .query_events(EventQuery {
            kind: query.kind,
            subject: query.subject,
            since: None,
            until: query.until,
            limit: Some(query.limit.unwrap_or(50)),
            cursor: query.since.or(query.cursor),
        })
        .await?;
    Ok(Json(page.into()))
}

/// Legacy-compatible alias of `/api/v1/timeline`, kept in OpenAPI so generated
/// clients see every route actually served by the router.
#[utoipa::path(
    get,
    path = "/api/v1/events",
    tag = "Timeline",
    params(TimelineQuery),
    responses((status = 200, description = "Event page", body = dto::EventPage))
)]
pub async fn events(
    state: State<AppState>,
    query: Query<TimelineQuery>,
) -> Result<Json<dto::EventPage>, ApiError> {
    timeline(state, query).await
}
