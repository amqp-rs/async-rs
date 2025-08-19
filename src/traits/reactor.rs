//! A collection of traits to define a common interface across reactors

use crate::{sys::IO, util::IOHandle};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    io,
    net::SocketAddr,
    ops::Deref,
    time::{Duration, Instant},
};

/// A common interface for performing actions on a reactor
pub trait Reactor {
    /// Register a synchronous handle, returning an asynchronous one
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
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
    fn tcp_connect(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static>>
    + Send
    + 'static
    where
        Self: Sized;
}

impl<R: Deref> Reactor for R
where
    R::Target: Reactor + Sized,
{
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        self.deref().register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> + Send + 'static {
        self.deref().sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        self.deref().interval(dur)
    }

    fn tcp_connect(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static>>
    + Send
    + 'static {
        self.deref().tcp_connect(addr)
    }
}
