use async_rs::{Runtime, TokioRuntime, traits::*};
use std::{io, sync::Arc, time::Duration};

async fn get_a(rt: Arc<TokioRuntime>) -> io::Result<u32> {
    rt.clone()
        .spawn_blocking(move || rt.block_on(async { Ok(12) }))
        .await
}

async fn get_b(rt: Arc<TokioRuntime>) -> io::Result<u32> {
    rt.spawn(async { Ok(30) }).await
}

async fn tokio_main() -> io::Result<()> {
    let rt = Arc::new(Runtime::tokio());
    let a = get_a(rt.clone()).await?;
    let b = get_b(rt.clone()).await?;
    rt.sleep(Duration::from_millis(500)).await;
    assert_eq!(a + b, 42);
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tokio_main().await
}

#[tokio::test]
async fn tokio() -> io::Result<()> {
    tokio_main().await
}
