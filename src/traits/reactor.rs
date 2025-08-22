//! A collection of traits to define a common interface across reactors

use crate::{sys::AsSysFd, traits::AsyncToSocketAddrs};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    io::{self, Read, Write},
    net::SocketAddr,
    ops::Deref,
    time::{Duration, Instant},
};

/// A common interface for performing actions on a reactor
pub trait Reactor {
    /// The type representing a TCP stream (after tcp_connect) for this reactor
    type TcpStream: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    /// Register a synchronous handle, returning an asynchronous one
    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static>
    where
        Self: Sized;

    /// Sleep for the given duration
    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> + Send + 'static
    where
        Self: Sized;

    /// Stream that yields at every given interval
    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static
    where
        Self: Sized;

    /// Create a TcpStream by connecting to a remote host
    fn tcp_connect<A: AsyncToSocketAddrs + Send>(
        &self,
        addrs: A,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send
    where
        Self: Sync + Sized,
    {
        async move {
            let mut err = None;
            for addr in addrs.to_socket_addrs().await? {
                match self.tcp_connect_addr(addr).await {
                    Ok(stream) => return Ok(stream),
                    Err(e) => err = Some(e),
                }
            }
            Err(err.unwrap_or_else(|| {
                io::Error::new(io::ErrorKind::AddrNotAvailable, "couldn't resolve host")
            }))
        }
    }

    /// Create a TcpStream by connecting to a remote host
    fn tcp_connect_addr(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static
    where
        Self: Sized;
}

impl<R: Deref> Reactor for R
where
    R::Target: Reactor + Sized,
{
    type TcpStream = <<R as Deref>::Target as Reactor>::TcpStream;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        self.deref().register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> + Send + 'static {
        self.deref().sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        self.deref().interval(dur)
    }

    fn tcp_connect_addr(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        self.deref().tcp_connect_addr(addr)
    }
}
