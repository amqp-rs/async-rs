use crate::{
    AsyncIOHandle, AsyncToSocketAddrs, Executor, IOHandle, Reactor, RuntimeKit, Task, sys::IO,
};
use async_trait::async_trait;
use futures_core::Stream;
use std::{
    fmt,
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
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

    /// Asynchronously resolve the given domain name
    pub fn to_socket_addrs<A: ToSocketAddrs + Send + 'static>(
        &self,
        addrs: A,
    ) -> impl AsyncToSocketAddrs
    where
        <A as std::net::ToSocketAddrs>::Iter: Iterator<Item = SocketAddr> + Send + 'static,
    {
        SocketAddrsResolver {
            runtime: self,
            addrs,
        }
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

#[async_trait]
impl<RK: RuntimeKit + Sync + 'static> Reactor for Runtime<RK> {
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send> {
        self.kit.register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> {
        self.kit.sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        self.kit.interval(dur)
    }

    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send> {
        self.kit.tcp_connect(addr).await
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

impl<E: Executor + Sync + fmt::Debug, R: Reactor + Sync + fmt::Debug> RuntimeKit
    for RuntimeParts<E, R>
{
}

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

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> {
        self.reactor.sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        self.reactor.interval(dur)
    }

    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send> {
        self.reactor.tcp_connect(addr).await
    }
}

struct SocketAddrsResolver<'a, RK: RuntimeKit + 'static, A: ToSocketAddrs + Send + 'static> {
    runtime: &'a Runtime<RK>,
    addrs: A,
}

impl<'a, RK: RuntimeKit + 'static, A: ToSocketAddrs + Send + 'static> AsyncToSocketAddrs
    for SocketAddrsResolver<'a, RK, A>
where
    <A as ToSocketAddrs>::Iter: Iterator<Item = SocketAddr> + Send + 'static,
{
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send>> + Send {
        let SocketAddrsResolver { runtime, addrs } = self;
        runtime.spawn_blocking(move || addrs.to_socket_addrs())
    }
}
