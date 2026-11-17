use crate::router::AppState;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
};
use hq_store::Patch;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Deserialize, Default)]
pub struct SseQuery {
    #[serde(default)]
    since: Option<i64>,
}

pub async fn sse_handler(
    State(state): State<AppState>,
    Query(q): Query<SseQuery>,
    headers: HeaderMap,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let store = state.store.clone();
    let ring = state.ring.clone();

    // determine cursor
    let mut cursor = q.since.unwrap_or(0);
    if let Some(v) = headers.get("last-event-id")
        && let Ok(s) = v.to_str()
        && let Ok(n) = s.parse::<i64>()
    {
        cursor = n;
    }

    // initial replay
    let replay = store
        .read_since(hq_core::Seq(cursor), 1001)
        .await
        .unwrap_or_default();
    let rx = store.patches();
    // If backlog truncated, still send what we have; client may request snapshot via WS. SSE keeps streaming.

    let stream = futures::stream::unfold(
        (replay, rx, ring, store, cursor),
        |(mut replay, mut rx, ring, store, last)| async move {
            // replay backlog first
            if !replay.is_empty() {
                let ev = replay.remove(0);
                let patch = hq_store::patch_for_event(&ev);
                let data = serde_json::to_string(&sse_patch_json(&patch)).unwrap_or_default();
                let e = Event::default().id(patch.seq.0.to_string()).data(data);
                return Some((Ok(e), (replay, rx, ring, store, patch.seq.0)));
            }
            // live
            match rx.recv().await {
                Ok(patch) => {
                    if patch.seq.0 <= last {
                        // skip duplicate
                        return Some((
                            Ok(Event::default().comment("skip")),
                            (Vec::new(), rx, ring, store, last),
                        ));
                    }
                    let data = serde_json::to_string(&sse_patch_json(&patch)).unwrap_or_default();
                    let e = Event::default().id(patch.seq.0.to_string()).data(data);
                    Some((Ok(e), (Vec::new(), rx, ring, store, patch.seq.0)))
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // fail-closed: close stream
                    None
                }
                Err(_) => None,
            }
        },
    );

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

fn sse_patch_json(patch: &Patch) -> serde_json::Value {
    let upserts: Vec<serde_json::Value> = patch
        .upserts
        .iter()
        .map(|u| match u {
            hq_store::PatchEntity::Agent(c) => serde_json::json!({
                "id": c.id,
                "name": c.name,
                "status": serde_json::to_value(c.status).unwrap_or_default(),
            }),
            hq_store::PatchEntity::Alert(id) => serde_json::json!({"id": id}),
            hq_store::PatchEntity::Usage(id) => serde_json::json!({"id": id}),
            hq_store::PatchEntity::Connector(id) => serde_json::json!({"id": id}),
            hq_store::PatchEntity::Approval(id) => serde_json::json!({"id": id}),
            hq_store::PatchEntity::Task(id) => serde_json::json!({"id": id}),
        })
        .collect();
    serde_json::json!({
        "seq": patch.seq.0,
        "upserts": upserts,
        "deletes": patch.deletes.iter().map(|d| serde_json::json!({"kind":d.kind,"id":d.id})).collect::<Vec<_>>(),
    })
}
