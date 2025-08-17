use async_rs::{Runtime, TokioRuntime, traits::*, util::IOHandle};
use futures_io::AsyncRead;
use std::{
    io,
    net::TcpListener,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

async fn listener(rt: Arc<TokioRuntime>) -> io::Result<TcpListener> {
    rt.spawn_blocking(|| TcpListener::bind(("127.0.0.1", 7654)))
        .await
}

/*
async fn sender(rt: Arc<TokioRuntime>) -> io::Result<impl AsyncIOHandle + Send> {
    rt.tcp_connect(([127, 0, 0, 1], 7654).into()).await
}
*/

fn send(mut stream: impl AsyncIOHandle + Unpin) -> io::Result<()> {
    let mut context = Context::from_waker(Waker::noop());
    match Pin::new(&mut stream).poll_write(&mut context, b"Hello, world!") {
        Poll::Pending => panic!("Could not write"),
        Poll::Ready(res) => assert_eq!(res?, 13),
    };
    Ok(())
}

async fn tokio_main() -> io::Result<()> {
    let rt = Arc::new(Runtime::tokio());
    let listener = listener(rt.clone()).await?;
    //let sender = sender(rt.clone()).await?;
    let sender = rt.tcp_connect(([127, 0, 0, 1], 7654).into()).await?;
    let stream = rt
        .spawn_blocking(move || listener.incoming().next().unwrap())
        .await?;
    let mut stream = rt.register(IOHandle::new(stream))?;
    let mut buf = vec![0u8; 13];
    let mut context = Context::from_waker(Waker::noop());
    send(sender)?;
    match Pin::new(&mut stream).poll_read(&mut context, &mut buf[..]) {
        Poll::Pending => panic!("Could not read"),
        Poll::Ready(res) => assert_eq!(res?, 13),
    };
    assert_eq!(String::from_utf8(buf).unwrap().as_str(), "Hello, world!");
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
