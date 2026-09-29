use async_rs::{Runtime, TokioRuntime, traits::*};
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    future::poll_fn,
    io::{self, ErrorKind},
    net::{SocketAddr, TcpListener},
    pin::Pin,
};

const MESSAGE: &[u8] = b"Hello, world!";

async fn listener(rt: &TokioRuntime) -> io::Result<TcpListener> {
    rt.spawn_blocking(|| TcpListener::bind(("127.0.0.1", 0)))
        .await
}

async fn sender(rt: &TokioRuntime, addr: SocketAddr) -> io::Result<impl AsyncWrite + Send + Unpin> {
    rt.tcp_connect(addr).await
}

async fn send(mut stream: impl AsyncWrite + Unpin) -> io::Result<()> {
    let mut remaining = MESSAGE;
    while !remaining.is_empty() {
        let written = poll_fn(|cx| Pin::new(&mut stream).poll_write(cx, remaining)).await?;
        if written == 0 {
            return Err(ErrorKind::WriteZero.into());
        }
        remaining = &remaining[written..];
    }
    poll_fn(|cx| Pin::new(&mut stream).poll_flush(cx)).await
}

async fn receive(mut stream: impl AsyncRead + Unpin, buf: &mut [u8]) -> io::Result<()> {
    let mut read = 0;
    while read < buf.len() {
        let count = poll_fn(|cx| Pin::new(&mut stream).poll_read(cx, &mut buf[read..])).await?;
        if count == 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        read += count;
    }
    Ok(())
}

async fn tokio_main(rt: &TokioRuntime) -> io::Result<()> {
    let listener = listener(rt).await?;
    let sender = sender(rt, listener.local_addr()?).await?;
    let stream = rt
        .spawn_blocking(move || {
            let (stream, _) = listener.accept()?;
            // The tokio reactor takes this as given rather than doing it itself: a blocking socket
            // would block the executor thread inside the read instead of reporting WouldBlock.
            stream.set_nonblocking(true)?;
            io::Result::Ok(stream)
        })
        .await?;
    let stream = rt.register(stream)?;
    let mut buf = [0u8; MESSAGE.len()];
    send(sender).await?;
    receive(stream, &mut buf).await?;
    assert_eq!(&buf, MESSAGE);
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let rt = Runtime::tokio_current();
    tokio_main(&rt).await
}
