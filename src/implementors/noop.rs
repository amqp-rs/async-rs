//! noop implementation of async runtime definition traits

use crate::{
    Runtime,
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit, Task},
    util::{DummyIO, DummyStream},
};
use async_trait::async_trait;
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    future::Future,
    io::{self, Read, Write},
    marker::PhantomData,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// Type alias for the noop runtime
pub type NoopRuntime = Runtime<Noop>;

impl NoopRuntime {
    /// Create a new NoopRuntime
    pub fn noop() -> Self {
        Self::new(Noop)
    }
}

/// Dummy object implementing async common interfaces on top of smol
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Noop;

struct NTask<T: Send + 'static>(PhantomData<T>);

impl RuntimeKit for Noop {}

impl Executor for Noop {
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        // We cannot fake something unless we require T: Default, which we don't want.
        // Let's get a minimalist implementation for this one.
        futures_executor::block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        _f: F,
    ) -> impl Task<T> + 'static {
        NTask(PhantomData)
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        _f: F,
    ) -> impl Task<T> + 'static {
        NTask(PhantomData)
    }
}

#[async_trait]
impl<T: Send + 'static> Task<T> for NTask<T> {
    async fn cancel(&mut self) -> Option<T> {
        None
    }
}

impl<T: Send + 'static> Future for NTask<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

impl Reactor for Noop {
    type TcpStream = DummyIO;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        _socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        Ok(DummyIO)
    }

    fn sleep(&self, _dur: Duration) -> impl Future<Output = ()> + Send + 'static {
        async {}
    }

    fn interval(&self, _dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        DummyStream(PhantomData)
    }

    fn tcp_connect_addr(
        &self,
        _addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        async { Ok(DummyIO) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyn_compat() {
        struct Test {
            _executor: Box<dyn Executor>,
            _reactor: Box<dyn Reactor<TcpStream = DummyIO>>,
            _kit: Box<dyn RuntimeKit<TcpStream = DummyIO>>,
            _task: Box<dyn Task<String>>,
        }

        let _ = Test {
            _executor: Box::new(Noop),
            _reactor: Box::new(Noop),
            _kit: Box::new(Noop),
            _task: Box::new(NTask(PhantomData)),
        };
    }

    #[test]
    fn auto_traits() {
        use crate::util::test::*;
        let runtime = Runtime::noop();
        assert_send(&runtime);
        assert_sync(&runtime);
        assert_clone(&runtime);
    }
}
