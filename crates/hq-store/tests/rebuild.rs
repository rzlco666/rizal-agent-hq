#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_store::*;

#[tokio::test]
async fn rebuild_reproduces_read_model() {
    let (store, dir) = test_store().await;
    seed_events(&store).await;
    let before = snapshot_json(&store).await;
    let report = store
        .rebuild(RebuildOptions::shadow(dir.path().join("shadow")))
        .await
        .unwrap();
    assert!(report.events > 0);
    let store2 = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let after = snapshot_json(&store2).await;
    assert_eq!(before, after);
}

#[tokio::test]
async fn rebuild_does_not_write_events() {
    let (store, _dir) = test_store().await;
    let n = store.count_events().await.unwrap();
    store
        .rebuild(RebuildOptions::in_place_test())
        .await
        .unwrap();
    assert_eq!(store.count_events().await.unwrap(), n);
}
