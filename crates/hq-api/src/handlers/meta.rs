use crate::{error::ApiError, state::AppState};
use axum::{Json, extract::State};
use utoipa::OpenApi;

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
