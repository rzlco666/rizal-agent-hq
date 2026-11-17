#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_store::*;

#[tokio::test]
async fn open_applies_schema_and_pragmas() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let h = store.health().await.unwrap();
    assert_eq!(h.journal_mode, "wal");
    assert!(h.tables.contains(&"events".to_string()));
    assert!(h.tables.contains(&"agents".to_string()));
    assert!(h.tables.contains(&"usage_minute".to_string()));
    assert_eq!(h.migrations, 1);
}

#[tokio::test]
async fn last_seq_propagates_archive_query_failure() {
    let dir = tempfile::tempdir().unwrap();
    let opts = StoreOptions::for_dir(dir.path());
    let store = Store::open(opts.clone()).await.unwrap();
    let conn = rusqlite::Connection::open(&opts.path).unwrap();
    conn.execute_batch("DROP TABLE events_archive").unwrap();

    assert!(store.last_seq().await.is_err());
}
