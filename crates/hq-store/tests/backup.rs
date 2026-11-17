#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_store::*;

#[tokio::test]
async fn backup_opens_and_counts_match() {
    let (store, _dir) = test_store().await;
    seed_events(&store).await;
    let out = tempfile::tempdir().unwrap().path().join("hq.db");
    store.backup(&out).await.unwrap();
    let copy = Store::open(StoreOptions::at_file(&out)).await.unwrap();
    assert_eq!(
        store.count_events().await.unwrap(),
        copy.count_events().await.unwrap()
    );
}
