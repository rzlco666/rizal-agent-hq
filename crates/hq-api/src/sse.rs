use crate::router::AppState;
use axum::{
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use hq_store::Patch;
use serde::Deserialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Deserialize, Default)]
pub struct SseQuery {
    #[serde(default)]
    since: Option<i64>,
}

enum Phase {
    Replay(Vec<Patch>),
    Snapshot(i64),
    Live,
    Done,
}

pub async fn sse_handler(
    State(state): State<AppState>,
    Query(q): Query<SseQuery>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    if headers.contains_key("x-forwarded-for") {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !addr.ip().is_loopback() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let rx = state.store.patches();

    let mut cursor = q.since.unwrap_or(0);
    if let Some(v) = headers.get("last-event-id")
        && let Ok(s) = v.to_str()
        && let Ok(n) = s.parse::<i64>()
    {
        cursor = n;
    }

    let last_seq = match state.store.last_seq().await {
        Ok(s) => s.0,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let replay_events = match state.store.read_since(hq_core::Seq(cursor), 1001).await {
        Ok(v) => v,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let truncated = replay_events.len() >= 1000
        || (cursor == 0 && last_seq > 1000)
        || (!replay_events.is_empty() && replay_events[0].seq.0 != cursor + 1 && cursor > 0);

    let replay_patches: Vec<Patch> = replay_events
        .into_iter()
        .map(|e| hq_store::patch_for_event(&e))
        .collect();

    let initial_phase = if truncated {
        Phase::Snapshot(last_seq)
    } else {
        Phase::Replay(replay_patches)
    };

    let stream = futures::stream::unfold(
        (initial_phase, rx, last_seq),
        |(phase, mut rx, last_seq)| async move {
            match phase {
                Phase::Replay(mut patches) => {
                    if let Some(p) = patches.first().cloned() {
                        patches.remove(0);
                        let data = serde_json::to_string(&sse_patch_json(&p)).unwrap_or_default();
                        let ev = Event::default().id(p.seq.0.to_string()).data(data);
                        Some((
                            Ok::<_, Infallible>(ev),
                            (Phase::Replay(patches), rx, p.seq.0),
                        ))
                    } else {
                        Some((
                            Ok::<_, Infallible>(Event::default().comment("")),
                            (Phase::Live, rx, last_seq),
                        ))
                    }
                }
                Phase::Snapshot(seq) => {
                    let ev = Event::default().data(
                        serde_json::json!({"type":"snapshot_required","seq":seq}).to_string(),
                    );
                    Some((Ok::<_, Infallible>(ev), (Phase::Done, rx, seq)))
                }
                Phase::Live => match rx.recv().await {
                    Ok(patch) => {
                        let data =
                            serde_json::to_string(&sse_patch_json(&patch)).unwrap_or_default();
                        let ev = Event::default().id(patch.seq.0.to_string()).data(data);
                        Some((Ok::<_, Infallible>(ev), (Phase::Live, rx, patch.seq.0)))
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        None // close stream; client reconnects with cursor
                    }
                    Err(_) => None,
                },
                Phase::Done => None,
            }
        },
    );

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("heartbeat"),
        )
        .into_response()
}

fn sse_patch_json(patch: &Patch) -> serde_json::Value {
    let upserts: Vec<serde_json::Value> = patch
        .upserts
        .iter()
        .map(|u| match u {
            hq_store::PatchEntity::Agent(c) => serde_json::json!({
                "type": "agent",
                "id": c.id,
                "name": c.name,
                "status": serde_json::to_value(c.status).unwrap_or_default(),
            }),
            hq_store::PatchEntity::Alert(id, payload) => {
                serde_json::json!({"type":"alert","id": id, "payload": payload})
            }
            hq_store::PatchEntity::Usage(id, payload) => {
                serde_json::json!({"type":"usage","id": id, "payload": payload})
            }
            hq_store::PatchEntity::Connector(id, payload) => {
                serde_json::json!({"type":"connector","id": id, "payload": payload})
            }
            hq_store::PatchEntity::Approval(id, payload) => {
                serde_json::json!({"type":"approval","id": id, "payload": payload})
            }
            hq_store::PatchEntity::Task(id, payload) => {
                serde_json::json!({"type":"task","id": id, "payload": payload})
            }
        })
        .collect();
    serde_json::json!({
        "seq": patch.seq.0,
        "upserts": upserts,
        "deletes": patch.deletes.iter().map(|d| serde_json::json!({"kind":d.kind,"id":d.id})).collect::<Vec<_>>(),
    })
}
