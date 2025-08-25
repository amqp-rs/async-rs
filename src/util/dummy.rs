use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    io,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

/// A dummy struct implementing Async IO traits
#[derive(Debug)]
pub struct DummyIO;

impl AsyncRead for DummyIO {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Pending
    }
}

impl AsyncWrite for DummyIO {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Pending
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

/// A dummy struct implementing Stream
#[derive(Debug)]
pub struct DummyStream<T>(pub PhantomData<T>);

impl<T> Stream for DummyStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}
