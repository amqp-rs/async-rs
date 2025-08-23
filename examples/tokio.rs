use async_rs::{Runtime, TokioRuntime, traits::*};
use std::{io, time::Duration};

async fn get_a(rt: &TokioRuntime) -> io::Result<u32> {
    rt.spawn_blocking(|| Ok(12)).await
}

async fn get_b(rt: &TokioRuntime) -> io::Result<u32> {
    rt.spawn(async { Ok(30) }).await
}

async fn tokio_main(rt: &TokioRuntime) -> io::Result<()> {
    let a = get_a(rt).await?;
    let b = get_b(rt).await?;
    rt.sleep(Duration::from_millis(500)).await;
    assert_eq!(a + b, 42);
    Ok(())
}

fn main() -> io::Result<()> {
    let rt = Runtime::tokio()?;
    rt.block_on(tokio_main(&rt))
}

#[test]
fn tokio() -> io::Result<()> {
    let rt = Runtime::tokio()?;
    rt.block_on(tokio_main(&rt))
}
