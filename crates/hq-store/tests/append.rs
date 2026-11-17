#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_store::*;

#[tokio::test]
async fn append_and_project_are_atomic() {
    let (store, _dir) = test_store().await;
    store.test_fail_after_insert(true);
    let err = store
        .append(vec![agent_observed("agent:omp", "bekerja")])
        .await;
    assert!(err.is_err());
    assert_eq!(store.count_events().await.unwrap(), 0);
    assert!(store.agents().await.unwrap().is_empty());
}

#[tokio::test]
async fn duplicate_idempotency_key_is_skipped() {
    let (store, _dir) = test_store().await;
    let d = heartbeat("agent:muse", "hb-1");
    assert_eq!(store.append(vec![d.clone()]).await.unwrap().len(), 1);
    assert_eq!(store.append(vec![d]).await.unwrap().len(), 0);
    assert_eq!(store.count_events().await.unwrap(), 1);
}

#[tokio::test]
async fn seq_is_monotonic() {
    let (store, _dir) = test_store().await;
    let e = store
        .append(vec![
            heartbeat("agent:muse", "a"),
            heartbeat("agent:muse", "b"),
        ])
        .await
        .unwrap();
    assert!(e[0].seq.0 < e[1].seq.0);
}
