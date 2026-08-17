use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    io,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

/// A dummy struct implementing Async IO traits
///
/// Every operation is `Poll::Pending`, forever: nothing is ever read, written, flushed or closed.
/// No waker is registered either, so a task waiting on one cannot even be woken to give up.
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
///
/// Like [`DummyIO`], it is `Poll::Pending` forever and registers no waker: the stream neither
/// yields an item nor ends.
#[derive(Debug)]
pub struct DummyStream<T>(pub PhantomData<T>);

impl<T> Stream for DummyStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}
