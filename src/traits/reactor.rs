//! A collection of traits to define a common interface across reactors

use crate::{sys::IO, util::IOHandle};
use async_trait::async_trait;
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    io,
    net::SocketAddr,
    ops::Deref,
    time::{Duration, Instant},
};

/// A common interface for performing actions on a reactor
#[async_trait]
pub trait Reactor {
    /// Register a synchronous handle, returning an asynchronous one
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send>
    where
        Self: Sized;

    /// Sleep for the given duration
    fn sleep(&self, dur: Duration) -> impl Future<Output = ()>
    where
        Self: Sized;

    /// Stream that yields at every given interval
    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant>
    where
        Self: Sized;

    /// Create a TcpStream by connecting to a remote host
    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send>
    where
        Self: Sized;
}

#[async_trait]
impl<R: Deref + Sync> Reactor for R
where
    R::Target: Reactor + Sized,
{
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send> {
        self.deref().register(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> {
        self.deref().sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        self.deref().interval(dur)
    }

    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send> {
        self.deref().tcp_connect(addr).await
    }
}

/// A trait representing an asynchronous IO handle
pub trait AsyncIOHandle: AsyncRead + AsyncWrite {}
impl<H: AsyncRead + AsyncWrite> AsyncIOHandle for H {}
