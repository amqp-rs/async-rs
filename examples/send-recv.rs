use async_rs::{Runtime, TokioRuntime, traits::*};
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    io,
    net::TcpListener,
    pin::Pin,
    task::{Context, Poll, Waker},
};

async fn listener(rt: &TokioRuntime) -> io::Result<TcpListener> {
    rt.spawn_blocking(|| TcpListener::bind(("127.0.0.1", 7654)))
        .await
}

async fn sender(rt: &TokioRuntime) -> io::Result<impl AsyncRead + AsyncWrite + Send + 'static> {
    rt.tcp_connect(([127, 0, 0, 1], 7654)).await
}

fn send(mut stream: impl AsyncRead + AsyncWrite + Unpin) -> io::Result<()> {
    let mut context = Context::from_waker(Waker::noop());
    match Pin::new(&mut stream).poll_write(&mut context, b"Hello, world!") {
        Poll::Pending => panic!("Could not write"),
        Poll::Ready(res) => assert_eq!(res?, 13),
    };
    match Pin::new(&mut stream).poll_flush(&mut context) {
        Poll::Pending => panic!("Could not flush"),
        Poll::Ready(res) => res,
    }
}

async fn tokio_main(rt: &TokioRuntime) -> io::Result<()> {
    let listener = listener(rt).await?;
    let sender = sender(rt).await?;
    let stream = rt
        .spawn_blocking(move || listener.incoming().next().unwrap())
        .await?;
    let mut stream = rt.register(stream)?;
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
    let rt = Runtime::tokio_current();
    tokio_main(&rt).await
}
