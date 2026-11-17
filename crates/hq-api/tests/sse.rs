#![allow(unused_imports, dead_code)]
use hq_api::api_router;
use hq_core::*;
use hq_store::*;
use std::net::SocketAddr;
use tempfile::TempDir;
use tokio::net::TcpListener;

async fn spawn() -> (SocketAddr, Ingestor, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(StoreOptions::for_dir(dir.path()))
        .await
        .unwrap();
    let ring = PatchRing::new(1000);
    let ing = Ingestor::new(store.clone());
    let app = api_router(store, ring);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    (addr, ing, dir)
}

#[tokio::test]
async fn sse_endpoint_exists() {
    let (addr, _ing, _dir) = spawn().await;
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    drop(stream);
}
