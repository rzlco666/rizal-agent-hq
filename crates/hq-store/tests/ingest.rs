#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_store::*;

#[tokio::test]
async fn ingest_redacts_before_persist() {
    let (store, _dir) = test_store().await;
    let ing = Ingestor::new(store.clone());
    let mut d = log_with_raw("agent:omp", "menjalankan tes", "token ghp_abcdefghijklmnop");
    d.payload["sentence"]["params"]["detail"] = "Authorization: Bearer abc.def.ghi".into();
    let r = ing.ingest(vec![d]).await.unwrap();
    let ev = store.read_since(Seq(0), 10).await.unwrap();
    let text = serde_json::to_string(&ev[0].draft.payload).unwrap();
    assert!(!text.contains("ghp_"));
    assert!(!text.contains("abc.def.ghi"));
    assert_eq!(r.accepted.len(), 1);
}

#[tokio::test]
async fn invalid_event_is_rejected_and_not_stored() {
    let ing = test_ingestor().await;
    let mut d = heartbeat("agent:muse", "bad");
    d.payload = serde_json::json!({});
    assert!(ing.ingest(vec![d]).await.is_err());
    assert_eq!(ing.store().count_events().await.unwrap(), 0);
}

#[tokio::test]
async fn duplicate_webhook_is_noop() {
    let ing = test_ingestor().await;
    let d = webhook_draft("github", "delivery-1");
    let a = ing.ingest(vec![d.clone()]).await.unwrap();
    let b = ing.ingest(vec![d]).await.unwrap();
    assert_eq!(a.accepted.len(), 1);
    assert_eq!(b.accepted.len(), 0);
    assert_eq!(b.duplicates, 1);
    assert_eq!(ing.store().count_events().await.unwrap(), 1);
}

#[test]
fn ring_returns_only_newer_patches() {
    use hq_store::PatchRing;
    let mut ring = PatchRing::new(3);
    fn patch(seq: i64) -> Patch {
        Patch {
            seq: Seq(seq),
            upserts: vec![],
            deletes: vec![],
        }
    }
    ring.push(patch(1));
    ring.push(patch(2));
    ring.push(patch(3));
    ring.push(patch(4));
    assert_eq!(ring.since(Seq(2)).unwrap().len(), 2);
    assert!(ring.since(Seq(0)).is_none());
}

#[test]
fn ring_rejects_internal_sequence_hole() {
    let mut ring = PatchRing::new(3);
    for seq in [1, 3] {
        ring.push(Patch {
            seq: Seq(seq),
            upserts: vec![],
            deletes: vec![],
        });
    }
    assert!(ring.since(Seq(1)).is_none());
}

#[tokio::test]
async fn session_and_log_events_produce_redacted_task_patches() {
    let (store, _dir) = test_store().await;
    let session_id = "agent:omp/session/s1";
    let events = [
        (
            EventKind::SessionStarted,
            serde_json::json!({"started_at_ms": 1000}),
        ),
        (
            EventKind::SessionUpdated,
            serde_json::json!({"updated_at_ms": 2000}),
        ),
        (
            EventKind::SessionEnded,
            serde_json::json!({"ended_at_ms": 3000}),
        ),
        (
            EventKind::LogRecorded,
            serde_json::json!({"message":"token ghp_abcdefghijklmnop"}),
        ),
    ]
    .into_iter()
    .map(|(kind, payload)| EventDraft {
        event_ts_ms: 1000,
        kind,
        subject: Subject::Session(SessionId(session_id.to_string())),
        source: SourceId("test".to_string()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: None,
        payload,
    })
    .collect();
    let accepted = Ingestor::new(store).ingest(events).await.unwrap().accepted;

    for mut event in accepted {
        if event.draft.kind == EventKind::LogRecorded {
            event.draft.payload = serde_json::json!({"message":"token ghp_abcdefghijklmnop"});
        }
        let patch = patch_for_event(&event);
        assert_eq!(patch.upserts.len(), 1, "{:?}", event.draft.kind);
        match &patch.upserts[0] {
            PatchEntity::Task(id, payload) => {
                assert_eq!(id, session_id);
                assert!(!payload.to_string().contains("ghp_"));
                if event.draft.kind != EventKind::LogRecorded {
                    assert_eq!(payload, &event.draft.payload);
                }
            }
            other => panic!("expected task upsert, got {other:?}"),
        }
    }
}
