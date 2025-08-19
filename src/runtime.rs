use crate::{
    sys::IO,
    traits::{AsyncToSocketAddrs, Executor, Reactor, RuntimeKit, Task},
    util::IOHandle,
};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
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
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        self.kit.block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        self.kit.spawn(f)
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        self.kit.spawn_blocking(f)
    }
}

impl<RK: RuntimeKit + 'static> Reactor for Runtime<RK> {
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send> {
        self.kit.register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> {
        self.kit.sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        self.kit.interval(dur)
    }

    fn tcp_connect(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<impl AsyncRead + AsyncWrite + Send + 'static>> + Send {
        self.kit.tcp_connect(addr)
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
