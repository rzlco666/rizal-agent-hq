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
    Replay(std::collections::VecDeque<Patch>),
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

    let last_seq = state.store.last_seq().await.map(|s| s.0).ok();
    let replay_events = state.store.read_since(hq_core::Seq(cursor), 1001).await;

    let replay_last_seq = replay_events
        .as_ref()
        .ok()
        .and_then(|events| events.last().map(|event| event.seq.0))
        .unwrap_or(cursor);
    let snapshot_seq = last_seq.unwrap_or(cursor).max(replay_last_seq);
    let complete = last_seq.is_some_and(|watermark| cursor <= watermark)
        && replay_events.as_ref().is_ok_and(|events| {
            let Some(mut expected) = cursor.checked_add(1) else {
                return false;
            };
            for event in events {
                if event.seq.0 != expected {
                    return false;
                }
                let Some(next) = expected.checked_add(1) else {
                    return false;
                };
                expected = next;
            }
            events.len() <= 1000 && expected > last_seq.unwrap_or(cursor)
        });

    let initial_phase = if complete {
        Phase::Replay(
            replay_events
                .unwrap()
                .into_iter()
                .map(|e| hq_store::patch_for_event(&e))
                .collect(),
        )
    } else {
        Phase::Snapshot(snapshot_seq)
    };
    let replay_cursor = match &initial_phase {
        Phase::Replay(patches) => patches.back().map_or(cursor, |p| p.seq.0),
        _ => cursor,
    };

    let stream = sse_event_stream(
        state.store,
        initial_phase,
        rx,
        replay_cursor,
    );

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("heartbeat"),
        )
        .into_response()
}

fn sse_event_stream(
    store: hq_store::Store,
    initial_phase: Phase,
    rx: broadcast::Receiver<Patch>,
    replay_cursor: i64,
) -> impl futures::Stream<Item = Result<Event, Infallible>> {
    futures::stream::unfold(
        (initial_phase, rx, replay_cursor),
        move |(phase, mut rx, cursor)| {
            let store = store.clone();
            async move {
                match phase {
                    Phase::Replay(mut patches) => {
                        if let Some(patch) = patches.pop_front() {
                            let data = serde_json::to_string(&sse_patch_json(&patch))
                                .unwrap_or_default();
                            let event = Event::default().id(patch.seq.0.to_string()).data(data);
                            Some((
                                Ok(event),
                                (Phase::Replay(patches), rx, cursor),
                            ))
                        } else {
                            Some((
                                Ok(Event::default().comment("")),
                                (Phase::Live, rx, cursor),
                            ))
                        }
                    }
                    Phase::Snapshot(seq) => {
                        let event = snapshot_required_event(seq);
                        Some((Ok(event), (Phase::Done, rx, seq)))
                    }
                    Phase::Live => match rx.recv().await {
                        Ok(patch) if should_skip_live_patch(patch.seq.0, cursor) => Some((
                            Ok(Event::default().comment("")),
                            (Phase::Live, rx, cursor),
                        )),
                        Ok(patch) => {
                            let Some(expected) = cursor.checked_add(1) else {
                                return Some((
                                    Ok(snapshot_required_event(patch.seq.0)),
                                    (Phase::Done, rx, patch.seq.0),
                                ));
                            };
                            if patch.seq.0 != expected {
                                let watermark = store
                                    .last_seq()
                                    .await
                                    .map(|seq| seq.0)
                                    .unwrap_or(patch.seq.0);
                                return Some((
                                    Ok(snapshot_required_event(watermark)),
                                    (Phase::Done, rx, watermark),
                                ));
                            }
                            let data = serde_json::to_string(&sse_patch_json(&patch))
                                .unwrap_or_default();
                            let event = Event::default().id(patch.seq.0.to_string()).data(data);
                            Some((Ok(event), (Phase::Live, rx, patch.seq.0)))
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            let watermark = store.last_seq().await.map(|seq| seq.0).unwrap_or(cursor);
                            Some((
                                Ok(snapshot_required_event(watermark)),
                                (Phase::Done, rx, watermark),
                            ))
                        }
                        Err(broadcast::error::RecvError::Closed) => None,
                    },
                    Phase::Done => None,
                }
            }
        },
    )
}

fn snapshot_required_event(seq: i64) -> Event {
    Event::default().data(
        serde_json::json!({"type":"snapshot_required","seq":seq}).to_string(),
    )
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

fn should_skip_live_patch(patch_seq: i64, replay_cursor: i64) -> bool {
    patch_seq <= replay_cursor
}

#[cfg(test)]
mod tests {
    use super::should_skip_live_patch;

    #[test]
    fn live_patch_at_or_before_replay_cursor_is_skipped() {
        assert!(should_skip_live_patch(7, 7));
        assert!(should_skip_live_patch(6, 7));
        assert!(!should_skip_live_patch(8, 7));
    }

    #[tokio::test]
    async fn production_stream_emits_replay_once_then_contiguous_live_patch() {
        use super::{Phase, sse_event_stream};
        use axum::response::IntoResponse;
        use futures::StreamExt;
        use std::collections::VecDeque;

        let dir = tempfile::tempdir().unwrap();
        let store = hq_store::Store::open(hq_store::StoreOptions::for_dir(dir.path()))
            .await
            .unwrap();
        let rx = store.patches();
        let ing = hq_store::Ingestor::new(store.clone());
        let events = ing
            .ingest(
                (1..=2)
                    .map(|i| hq_core::EventDraft {
                        event_ts_ms: 1000,
                        kind: hq_core::EventKind::AgentSeen,
                        subject: hq_core::Subject::Agent(hq_core::AgentId("agent:muse".into())),
                        source: hq_core::SourceId("test".into()),
                        correlation_id: None,
                        causation_id: None,
                        idempotency_key: Some(format!("stream-{i}")),
                        payload: serde_json::json!({"seen_at_ms": 1000}),
                    })
                    .collect(),
            )
            .await
            .unwrap()
            .accepted;
        let replay = hq_store::patch_for_event(&events[0]);
        let stream = sse_event_stream(
            store,
            Phase::Replay(VecDeque::from([replay])),
            rx,
            events[0].seq.0,
        );
        let response = super::Sse::new(stream).into_response();
        let mut body = response.into_body().into_data_stream();
        let mut bytes = String::new();
        let mut ids = Vec::new();
        while let Some(Ok(frame)) = body.next().await {
            bytes.push_str(&String::from_utf8_lossy(&frame));
            ids = bytes
                .lines()
                .filter_map(|line| line.strip_prefix("id: "))
                .map(str::to_owned)
                .collect();
            if ids.last().is_some_and(|id| id == "2") {
                break;
            }
        }
        assert_eq!(ids, ["1", "2"], "{bytes}");
    }
}
