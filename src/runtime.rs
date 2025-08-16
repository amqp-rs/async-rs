use crate::{AsyncIOHandle, Executor, IOHandle, Reactor, RuntimeKit, Task, sys::IO};
use async_trait::async_trait;
use futures_core::Stream;
use std::{
    fmt::Debug,
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    time::{Duration, Instant},
};

/// A full-featured Runtime implementation
#[derive(Debug)]
pub struct Runtime<RK: RuntimeKit + 'static> {
    kit: RK,
}

impl<RK: RuntimeKit + 'static> Runtime<RK> {
    /// Create a new Runtime from a RuntimeKit
    pub fn new(kit: RK) -> Self {
        Self { kit }
    }
}

impl<RK: RuntimeKit + 'static> From<RK> for Runtime<RK> {
    fn from(kit: RK) -> Self {
        Self::new(kit)
    }
}

impl<RK: RuntimeKit + 'static> Executor for Runtime<RK> {
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T {
        self.kit.block_on(f)
    }

    fn spawn<T: Send + 'static>(
        &self,
        f: impl Future<Output = T> + Send + 'static,
    ) -> impl Task<T> {
        self.kit.spawn(f)
    }

    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> {
        self.kit.spawn_blocking(f)
    }
}

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

impl<E: Executor + Sync, R: Reactor + Sync> RuntimeKit for RuntimeParts<E, R> {}

impl<E: Executor, R: Reactor> Executor for RuntimeParts<E, R> {
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T {
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

#[async_trait]
impl<E: Executor + Sync, R: Reactor + Sync> Reactor for RuntimeParts<E, R> {
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send> {
        self.reactor.register(socket)
    }

    async fn sleep(&self, dur: Duration) {
        self.reactor.sleep(dur).await;
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        self.reactor.interval(dur)
    }

    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send> {
        self.reactor.tcp_connect(addr).await
    }
}
