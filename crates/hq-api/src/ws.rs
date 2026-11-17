use crate::router::AppState;
use axum::{
    extract::{ConnectInfo, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use hq_store::{Patch, PatchEntity, patch_for_event};
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
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // fail closed: reject proxied requests
    if headers.contains_key("x-forwarded-for") {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !addr.ip().is_loopback() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, q.since))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, state: AppState, since: i64) {
    let (mut sender, mut receiver) = socket.split();
    let store = state.store.clone();
    let ring = state.ring.clone();

    // subscribe first to avoid missing appends in the gap
    let mut rx = store.patches();

    let last_seq = match store.last_seq().await {
        Ok(seq) => seq.0,
        Err(_) => {
            send_snapshot_required(&mut sender, since).await;
            let _ = sender.close().await;
            return;
        }
    };
    let missed = match store.read_since(hq_core::Seq(since), 1001).await {
        Ok(events) => events,
        Err(_) => {
            send_snapshot_required(&mut sender, last_seq).await;
            let _ = sender.close().await;
            return;
        }
    };

    let Some(watermark) = replay_watermark(since, last_seq, missed.iter().map(|event| event.seq.0))
        .filter(|_| missed.len() <= 1000)
    else {
        send_snapshot_required(&mut sender, last_seq).await;
        let _ = sender.close().await;
        return;
    };

    let _ = sender
        .send(axum::extract::ws::Message::Text(
            json!({"type":"hello","seq":watermark}).to_string().into(),
        ))
        .await;

    for ev in &missed {
        let patch = patch_for_event(ev);
        let _ = sender
            .send(axum::extract::ws::Message::Text(
                patch_to_json(&patch).to_string().into(),
            ))
            .await;
    }
    let mut last_sent = watermark;

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(patch) if patch.seq.0 <= last_sent => continue,
                    Ok(patch) => {
                        if last_sent.checked_add(1) != Some(patch.seq.0) {
                            let watermark = store.last_seq().await.map(|seq| seq.0).unwrap_or(patch.seq.0);
                            send_snapshot_required(&mut sender, watermark).await;
                            let _ = sender.close().await;
                            return;
                        }
                        let _ = sender.send(axum::extract::ws::Message::Text(patch_to_json(&patch).to_string().into())).await;
                        last_sent = patch.seq.0;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let watermark = match store.last_seq().await {
                            Ok(seq) => seq.0,
                            Err(_) => {
                                send_snapshot_required(&mut sender, last_seq).await;
                                let _ = sender.close().await;
                                return;
                            }
                        };
                        let recovered = ring
                            .lock()
                            .ok()
                            .and_then(|g| g.since(hq_core::Seq(last_sent)));
                        let Some(patches) = recovered else {
                            send_snapshot_required(&mut sender, watermark).await;
                            let _ = sender.close().await;
                            return;
                        };
                        let Some(mut expected) = last_sent.checked_add(1) else {
                            send_snapshot_required(&mut sender, watermark).await;
                            let _ = sender.close().await;
                            return;
                        };
                        let mut contiguous = true;
                        for patch in &patches {
                            if patch.seq.0 != expected {
                                contiguous = false;
                                break;
                            }
                            let Some(next) = expected.checked_add(1) else {
                                contiguous = false;
                                break;
                            };
                            expected = next;
                        }
                        if !contiguous || expected.checked_sub(1).is_none_or(|seq| seq < watermark) {
                            send_snapshot_required(&mut sender, watermark).await;
                            let _ = sender.close().await;
                            return;
                        }
                        for patch in patches {
                            let _ = sender.send(axum::extract::ws::Message::Text(patch_to_json(&patch).to_string().into())).await;
                            last_sent = patch.seq.0;
                        }
                    }
                    Err(_) => break,
                }
            }
            r = receiver.next() => {
                if r.is_none() { break; }
            }
        }
    }
}

fn replay_watermark(
    since: i64,
    captured_watermark: i64,
    replay_seqs: impl IntoIterator<Item = i64>,
) -> Option<i64> {
    if since > captured_watermark {
        return None;
    }
    let mut expected = since.checked_add(1)?;
    let mut replay_watermark = since;
    for seq in replay_seqs {
        if seq != expected {
            return None;
        }
        replay_watermark = seq;
        expected = expected.checked_add(1)?;
    }
    (replay_watermark >= captured_watermark).then_some(replay_watermark.max(captured_watermark))
}

fn patch_to_json(patch: &Patch) -> serde_json::Value {
    let upserts: Vec<serde_json::Value> = patch
        .upserts
        .iter()
        .map(|u| match u {
            PatchEntity::Agent(id, event_kind, payload) => json!({
                "type": "agent",
                "id": id,
                "event_kind": event_kind.as_str(),
                "payload": payload,
            }),
            PatchEntity::Alert(id, event_kind, payload) => json!({
                "type":"alert",
                "id": id,
                "event_kind": event_kind.as_str(),
                "payload": payload
            }),
            PatchEntity::Usage(id, event_kind, payload) => json!({
                "type":"usage",
                "id": id,
                "event_kind": event_kind.as_str(),
                "payload": payload
            }),
            PatchEntity::Connector(id, event_kind, payload) => json!({
                "type":"connector",
                "id": id,
                "event_kind": event_kind.as_str(),
                "payload": payload
            }),
            PatchEntity::Approval(id, event_kind, payload) => json!({
                "type":"approval",
                "id": id,
                "event_kind": event_kind.as_str(),
                "payload": payload
            }),
            PatchEntity::Task(id, event_kind, payload) => json!({
                "type":"task",
                "id": id,
                "event_kind": event_kind.as_str(),
                "payload": payload
            }),
            PatchEntity::Log(id, event_kind, payload) => json!({
                "type":"log",
                "id": id,
                "event_kind": event_kind.as_str(),
                "payload": payload
            }),
        })
        .collect();
    json!({
        "type":"patch",
        "seq":patch.seq.0,
        "upserts": upserts,
        "deletes": patch.deletes.iter().map(|d| json!({"kind":d.kind,"id":d.id})).collect::<Vec<_>>(),
    })
}

async fn send_snapshot_required(
    sender: &mut futures::stream::SplitSink<
        axum::extract::ws::WebSocket,
        axum::extract::ws::Message,
    >,
    seq: i64,
) {
    let _ = sender
        .send(axum::extract::ws::Message::Text(
            json!({"type":"snapshot_required","seq":seq})
                .to_string()
                .into(),
        ))
        .await;
}
#[cfg(test)]
mod tests {
    #[test]
    fn replay_watermark_includes_contiguous_events_after_captured_watermark() {
        assert_eq!(super::replay_watermark(10, 12, [11, 12, 13]), Some(13));
    }
}
