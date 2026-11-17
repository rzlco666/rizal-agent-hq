#![allow(unused_imports, dead_code)]
mod common;
use common::*;
use hq_core::*;
use hq_store::*;

#[tokio::test]
async fn agent_observed_upserts_card() {
    let (store, _dir) = test_store().await;
    store
        .append(vec![agent_observed("agent:omp", "bekerja")])
        .await
        .unwrap();
    let a = store
        .agent(&AgentId::parse("agent:omp").unwrap(), None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.card.status, Status::Bekerja);
    assert_eq!(a.card.name, "OMP");
}

#[tokio::test]
async fn usage_is_aggregated_per_minute_and_day() {
    let (store, _dir) = test_store().await;
    store
        .append(vec![usage("agent:omp", 1_000, 1_700_000_000_000)])
        .await
        .unwrap();
    let r = store
        .usage(UsageQuery::agent_minutes("agent:omp"))
        .await
        .unwrap();
    assert_eq!(r.total.input_tokens, 1_000);
}

#[tokio::test]
async fn patches_are_emitted_after_commit() {
    let (store, _dir) = test_store().await;
    let mut rx = store.patches();
    store
        .append(vec![agent_observed("agent:omp", "bekerja")])
        .await
        .unwrap();
    let p = rx.recv().await.unwrap();
    assert!(p.upserts.iter().any(|u| matches!(u, PatchEntity::Agent(_))));
}

#[tokio::test]
async fn read_since_returns_in_seq_order() {
    let (store, _dir) = test_store().await;
    let e = store
        .append(vec![
            heartbeat("agent:muse", "a"),
            heartbeat("agent:muse", "b"),
        ])
        .await
        .unwrap();
    let got = store.read_since(Seq(0), 10).await.unwrap();
    assert_eq!(
        got.iter().map(|x| x.seq).collect::<Vec<_>>(),
        vec![e[0].seq, e[1].seq]
    );
}

#[tokio::test]
async fn read_since_does_not_decode_rows_after_limit() {
    let (store, dir) = test_store().await;
    let first = store
        .append(vec![heartbeat("agent:muse", "first")])
        .await
        .unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("hq.db")).unwrap();
    conn.execute(
        "INSERT INTO events (seq,id,event_ts_ms,ingest_ts_ms,kind,subject,source,schema_version,payload) \
         VALUES (2,'bad',1000,1000,'agent_seen','agent:muse','test',1,X'00')",
        [],
    )
    .unwrap();

    let got = store.read_since(Seq(0), 1).await.unwrap();

    assert_eq!(got.len(), 1);
    assert_eq!(got[0].seq, first[0].seq);
}
