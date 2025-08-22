use crate::{
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit, Task},
    util::SocketAddrsResolver,
};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    future::Future,
    io::{self, Read, Write},
    net::{SocketAddr, ToSocketAddrs},
    time::{Duration, Instant},
};

/// A full-featured Runtime implementation
#[derive(Debug)]
pub struct Runtime<RK: RuntimeKit> {
    kit: RK,
}

impl<RK: RuntimeKit> Runtime<RK> {
    /// Create a new Runtime from a RuntimeKit
    pub fn new(kit: RK) -> Self {
        Self { kit }
    }

    /// Asynchronously resolve the given domain name
    pub fn to_socket_addrs<A: ToSocketAddrs + Send + 'static>(
        &self,
        addrs: A,
    ) -> SocketAddrsResolver<'_, RK, A>
    where
        <A as std::net::ToSocketAddrs>::Iter: Send + 'static,
    {
        SocketAddrsResolver {
            runtime: self,
            addrs,
        }
    }
}

impl<RK: RuntimeKit> From<RK> for Runtime<RK> {
    fn from(kit: RK) -> Self {
        Self::new(kit)
    }
}

impl<RK: RuntimeKit> Executor for Runtime<RK> {
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

impl<RK: RuntimeKit> Reactor for Runtime<RK> {
    type TcpStream = <RK as Reactor>::TcpStream;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        self.kit.register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> + Send + 'static {
        self.kit.sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        self.kit.interval(dur)
    }

    fn tcp_connect_addr(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        self.kit.tcp_connect_addr(addr)
    }
}
