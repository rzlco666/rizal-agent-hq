use crate::{dto, error::ApiError, state::AppState};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use hq_store::Locale;
use serde::Deserialize;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TaskQuery {
    pub agent: Option<String>,
    pub phase: Option<String>,
    pub q: Option<String>,
    pub limit: Option<usize>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    tag = "Tasks",
    params(TaskQuery),
    responses((status = 200, description = "Task page", body = dto::TaskPage))
)]
pub async fn tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<dto::TaskPage>, ApiError> {
    let items = state
        .store
        .tasks(
            query.agent.as_deref(),
            query.phase.as_deref(),
            query.q.as_deref(),
            Locale::Id,
            query.limit.unwrap_or(50),
        )
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
    path = "/api/v1/tasks/{id}",
    tag = "Tasks",
    params(("id" = String, Path, description = "Task id")),
    responses((status = 200, description = "Task detail", body = dto::TaskView))
)]
pub async fn task_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<dto::TaskView>, ApiError> {
    let task = state
        .store
        .task(&id, Locale::Id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(task.into()))
}
