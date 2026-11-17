use crate::router::AppState;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::unfold;
use hq_store::Patch;
use std::convert::Infallible;
use std::time::Duration;

pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.store.patches();
    let stream = unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(patch) => {
                let data = serde_json::to_string(&patch_to_json(&patch)).unwrap_or_default();
                let ev = Event::default().id(patch.seq.0.to_string()).data(data);
                Some((Ok(ev), rx))
            }
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

fn patch_to_json(patch: &Patch) -> serde_json::Value {
    let upserts: Vec<serde_json::Value> = patch
        .upserts
        .iter()
        .map(|u| match u {
            hq_store::PatchEntity::Agent(c) => serde_json::json!({"type":"agent","id":c.id}),
            _ => serde_json::json!({"type":"other"}),
        })
        .collect();
    serde_json::json!({
        "seq": patch.seq.0,
        "upserts": upserts,
        "deletes": patch.deletes.iter().map(|d| serde_json::json!({"kind":d.kind,"id":d.id})).collect::<Vec<_>>(),
    })
}
