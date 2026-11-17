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
        Patch { seq: Seq(seq), upserts: vec![], deletes: vec![] }
    }
    ring.push(patch(1)); ring.push(patch(2)); ring.push(patch(3)); ring.push(patch(4));
    assert_eq!(ring.since(Seq(2)).unwrap().len(), 2);
    assert!(ring.since(Seq(0)).is_none());
}
