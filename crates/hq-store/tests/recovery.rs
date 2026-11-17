#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_store::*;

#[tokio::test]
async fn restart_after_failed_commit_recovers() {
    let (store, dir) = test_store().await;
    store.test_fail_after_insert(true);
    let _ = store
        .append(vec![agent_observed("agent:omp", "bekerja")])
        .await;
    drop(store);
    let reopened = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    assert_eq!(reopened.count_events().await.unwrap(), 0);
    assert!(reopened.agents().await.unwrap().is_empty());
}
