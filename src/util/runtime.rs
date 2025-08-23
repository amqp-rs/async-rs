use crate::{
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit, Task},
};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    fmt,
    future::Future,
    io::{self, Read, Write},
    net::SocketAddr,
    time::{Duration, Instant},
};

/// Wrapper around separate Executor and Reactor implementing RuntimeKit
#[derive(Clone, Debug)]
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

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        self.executor.spawn(f)
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        self.executor.spawn_blocking(f)
    }
}

impl<E: Executor, R: Reactor> Reactor for RuntimeParts<E, R> {
    type TcpStream = R::TcpStream;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        self.reactor.register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> + Send + 'static {
        self.reactor.sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        self.reactor.interval(dur)
    }

    fn tcp_connect_addr(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        self.reactor.tcp_connect_addr(addr)
    }
}
