use crate::{
    sys::IO,
    traits::{Executor, Reactor, RuntimeKit, Task},
    util::IOHandle,
};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    fmt,
    future::Future,
    io,
    net::SocketAddr,
    time::{Duration, Instant},
};

/// Wrapper around separate Executor and Reactor implementing RuntimeKit
#[derive(Debug)]
pub struct RuntimeParts<E: Executor, R: Reactor> {
    executor: E,
    reactor: R,
}

impl<E: Executor, R: Reactor> RuntimeParts<E, R> {
    /// Create new RuntimeParts from separate Executor and Reactor
    pub fn new(executor: E, reactor: R) -> Self {
        Self { executor, reactor }
    }
}

impl<E: Executor + fmt::Debug, R: Reactor + fmt::Debug> RuntimeKit for RuntimeParts<E, R> {}

impl<E: Executor, R: Reactor> Executor for RuntimeParts<E, R> {
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        self.executor.block_on(f)
    }

    fn spawn<T: Send + 'static>(
        &self,
        f: impl Future<Output = T> + Send + 'static,
    ) -> impl Task<T> {
        self.executor.spawn(f)
    }

    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> {
        self.executor.spawn_blocking(f)
    }
}

impl<E: Executor, R: Reactor> Reactor for RuntimeParts<E, R> {
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send> {
        self.reactor.register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> {
        self.reactor.sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        self.reactor.interval(dur)
    }

    fn tcp_connect(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<impl AsyncRead + AsyncWrite + Send + 'static>> + Send {
        self.reactor.tcp_connect(addr)
    }
}
