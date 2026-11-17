use crate::router::AppState;
use axum::{
    extract::{ConnectInfo, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use hq_core::Event;
use hq_store::Patch;
use hq_store::PatchEntity;
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;
use tokio::sync::broadcast;

#[derive(Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    since: i64,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(q): Query<WsQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !addr.ip().is_loopback() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, q.since))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, state: AppState, since: i64) {
    let (mut sender, mut receiver) = socket.split();
    let store = state.store.clone();

    let last_seq = store.last_seq().await.map(|s| s.0).unwrap_or(0);
    let missed: Vec<hq_store::Event> = store
        .read_since(hq_core::Seq(since), 1000)
        .await
        .unwrap_or_default();

    let mut snapshot = false;
    if since > last_seq {
        snapshot = true;
    } else if since > 0 {
        if missed.is_empty() {
            if since < last_seq {
                snapshot = true;
            }
        } else if missed[0].seq.0 != since + 1 {
            snapshot = true;
        }
    }
    // also if since==0 but? no.

    if snapshot {
        let _ = sender
            .send(axum::extract::ws::Message::Text(
                json!({"type":"snapshot_required","seq":last_seq}).to_string(),
            ))
            .await;
        let _ = sender.close().await;
        while receiver.next().await.is_some() {}
        return;
    }

    // hello
    let _ = sender
        .send(axum::extract::ws::Message::Text(
            json!({"type":"hello","seq":last_seq}).to_string(),
        ))
        .await;


    // push missed as patches
    let mut last_sent = since;
    for ev in &missed {
        let patch = event_to_patch(ev);
        let _ = sender
            .send(axum::extract::ws::Message::Text(
                patch_to_json(&patch).to_string(),
            ))
            .await;
        last_sent = patch.seq.0;
    }

    // live stream
    let mut rx = store.patches();
    // skip already sent
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(patch) => {
                        if patch.seq.0 <= last_sent { continue; }
                        let _ = sender.send(axum::extract::ws::Message::Text(patch_to_json(&patch).to_string())).await;
                        last_sent = patch.seq.0;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            _ = receiver.next() => {
                // client closed or message; keep
            }
        }
    }
}

fn event_to_patch(ev: &Event) -> Patch {
    use hq_store::Patch;
    let mut upserts = Vec::new();
    match ev.draft.kind {
        hq_core::EventKind::AgentObserved
        | hq_core::EventKind::AgentSeen
        | hq_core::EventKind::AgentStatusChanged
        | hq_core::EventKind::AgentDisappeared => {
            let status = ev
                .draft
                .payload
                .get("status")
                .and_then(|v| v.as_str())
                .map(hq_store::status_from_str)
                .unwrap_or(hq_core::Status::Bekerja);
            let card = hq_store::AgentCard {
                id: ev.draft.subject.key(),
                name: hq_store::agent_name(&ev.draft.subject.key()),
                status,
            };
            upserts.push(PatchEntity::Agent(card));
        }
        _ => {}
    }
    Patch {
        seq: ev.seq,
        upserts,
        deletes: Vec::new(),
    }
}

fn patch_to_json(patch: &Patch) -> serde_json::Value {
    // Need serialize upserts. PatchEntity isn't serde. Build manually.
    let upserts: Vec<serde_json::Value> = patch
        .upserts
        .iter()
        .map(|u| match u {
            PatchEntity::Agent(c) => {
                json!({"type":"agent","id":c.id,"name":c.name,"status":format!("{:?}",c.status)})
            }
            _ => json!({"type":"other"}),
        })
        .collect();
    json!({
        "type":"patch",
        "seq":patch.seq.0,
        "upserts": upserts,
        "deletes": patch.deletes.iter().map(|d| json!({"kind":d.kind,"id":d.id})).collect::<Vec<_>>(),
    })
}
