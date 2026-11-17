use crate::{error::ApiError, state::AppState};
use axum::{Json, extract::State};
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DbHealth {
    pub ok: bool,
    pub journal_mode: String,
    pub wal_size_bytes: u64,
    pub tables: usize,
    pub migrations: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub ok: bool,
    pub seq: i64,
    pub last_seq: i64,
    pub integrity_ok: bool,
    pub uptime_secs: u64,
    pub db: DbHealth,
}

#[utoipa::path(get, path = "/api/v1/health", tag = "Meta", responses((status = 200, description = "Health", body = HealthResponse)))]
pub async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    let report = state.store.health().await?;
    let seq = state.store.last_seq().await?.0.max(report.last_seq);
    Ok(Json(HealthResponse {
        ok: report.integrity_ok,
        seq,
        last_seq: report.last_seq,
        integrity_ok: report.integrity_ok,
        uptime_secs: state.started_at.elapsed().as_secs(),
        db: DbHealth {
            ok: report.integrity_ok,
            journal_mode: report.journal_mode,
            wal_size_bytes: report.wal_size_bytes,
            tables: report.tables.len(),
            migrations: report.migrations,
        },
    }))
}

/// Legacy path kept for S03 probes; same payload as `/api/v1/health`.
pub async fn health_legacy(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, ApiError> {
    health(State(state)).await
}
