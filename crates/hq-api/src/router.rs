use crate::health::{HealthState, health};
use crate::sse;
use crate::ws;
use axum::{Router, routing::get};
use hq_store::{PatchRing, Store};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub ring: Arc<Mutex<PatchRing>>,
}

fn spawn_patch_forwarder(
    mut rx: tokio::sync::broadcast::Receiver<hq_store::Patch>,
    ring: Arc<Mutex<PatchRing>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(patch) => {
                    if let Ok(mut ring) = ring.lock() {
                        ring.push(patch);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

pub fn api_router(store: Store, ring: PatchRing) -> Router {
    let state = AppState {
        store: store.clone(),
        ring: Arc::new(Mutex::new(ring)),
    };
    // forward broadcast patches into ring
    drop(spawn_patch_forwarder(store.patches(), state.ring.clone()));

    let health_state = HealthState { store };

    Router::new()
        .route("/health", get(health).with_state(health_state))
        .route("/ws", get(ws::ws_handler))
        .route("/api/v1/events/stream", get(sse::sse_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::spawn_patch_forwarder;
    use hq_store::{Patch, PatchRing, Seq};
    use std::sync::{Arc, Mutex};
    use tokio::sync::broadcast;

    fn patch(seq: i64) -> Patch {
        Patch {
            seq: Seq(seq),
            upserts: Vec::new(),
            deletes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn forwarder_resumes_after_lag_and_pushes_later_patch() {
        let (tx, rx) = broadcast::channel(1);
        let ring = Arc::new(Mutex::new(PatchRing::new(2)));
        tx.send(patch(1)).unwrap();
        tx.send(patch(2)).unwrap();

        let forwarder = spawn_patch_forwarder(rx, ring.clone());
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if ring.lock().unwrap().since(Seq(2)).is_some() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("forwarder did not resume after lag");

        tx.send(patch(3)).unwrap();
        drop(tx);
        forwarder.await.unwrap();

        let ring = ring.lock().unwrap();
        assert!(ring.since(Seq(1)).is_none());
        let later = ring.since(Seq(2)).unwrap();
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].seq, Seq(3));
    }
}
