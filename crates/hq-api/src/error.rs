use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use hq_store::StoreError;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    pub detail: Option<serde_json::Value>,
    pub request_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden(String),
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict(String),
    #[error("invalid request")]
    BadRequest(String),
    #[error("internal error")]
    Internal(String),
}

impl ApiError {
    pub fn body(&self) -> ErrorBody {
        let code = match self {
            Self::Unauthorized => "unauthorized",
            Self::Forbidden(_) => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict(_) => "conflict",
            Self::BadRequest(_) => "invalid_request",
            Self::Internal(_) => "internal",
        };
        ErrorBody {
            code,
            message: self.to_string(),
            detail: match self {
                Self::Forbidden(detail)
                | Self::Conflict(detail)
                | Self::BadRequest(detail)
                | Self::Internal(detail) => Some(json!(detail)),
                _ => None,
            },
            request_id: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self.body())).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotFound => Self::NotFound,
            StoreError::Validation(detail) | StoreError::Core(detail) => Self::BadRequest(detail),
            other => Self::Internal(other.to_string()),
        }
    }
}
