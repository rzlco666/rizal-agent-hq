use crate::{dto, error::ApiError, state::AppState};
use axum::{Json, extract::State};

#[utoipa::path(
    get,
    path = "/api/v1/overview",
    tag = "Today",
    responses((status = 200, description = "Current operational snapshot", body = dto::Overview))
)]
pub async fn overview(State(state): State<AppState>) -> Result<Json<dto::Overview>, ApiError> {
    let summary = state.store.overview().await?;
    let cards = state.store.agents().await?;
    let seq = state.store.last_seq().await?.0;
    let mut working = 0;
    let mut waiting = 0;
    let mut offline = 0;
    for card in &cards {
        match card.card.status {
            hq_store::Status::Bekerja | hq_store::Status::Berpikir => working += 1,
            hq_store::Status::MenungguRizal => waiting += 1,
            hq_store::Status::Offline | hq_store::Status::Error => offline += 1,
            hq_store::Status::Santai => {}
        }
    }
    Ok(Json(dto::Overview {
        agents: cards.into_iter().map(|detail| detail.card.into()).collect(),
        counts: dto::Counts {
            working,
            waiting,
            offline,
            alerts_open: summary.alerts_open,
        },
        seq,
        usage_requests: summary.usage_requests,
    }))
}
