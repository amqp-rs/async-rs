use crate::{AsyncIOHandle, IOHandle, Reactor, sys::IO};
use async_io::{Async, Timer};
use async_trait::async_trait;
use futures_core::Stream;
use std::{
    future::Future,
    io,
    net::{SocketAddr, TcpStream},
    pin::Pin,
    task::{self, Context, Poll},
    time::{Duration, Instant},
};

/// Dummy object implementing reactor common interfaces on top of async-io
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncIO;

#[async_trait]
impl Reactor for AsyncIO {
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send> {
        Async::new(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> {
        TimerTask(Timer::after(dur))
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        Timer::interval(dur)
    }

    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send> {
        Async::<TcpStream>::connect(addr).await
    }
}

pub(crate) struct TimerTask<T: Future + Unpin>(pub(crate) T);

impl<T: Future + Unpin> Future for TimerTask<T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        task::ready!(Pin::new(&mut self.0).poll(cx));
        Poll::Ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyn_compat() {
        struct Test {
            _reactor: Box<dyn Reactor>,
        }

        let _ = Test {
            _reactor: Box::new(AsyncIO),
        };
    }
}
