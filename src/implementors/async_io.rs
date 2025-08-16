use crate::{AsyncIOHandle, IOHandle, Reactor, sys::IO};
use async_io::{Async, IoSafe, Timer};
use async_trait::async_trait;
use futures_core::Stream;
use std::{
    io,
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

/// Dummy object implementing reactor common interfaces on top of async-io
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncIO;

#[async_trait]
impl Reactor for AsyncIO {
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send> {
        Async::new(socket)
    }

    async fn sleep(&self, dur: Duration) {
        Timer::after(dur).await;
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        Timer::interval(dur)
    }

    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send> {
        Async::<TcpStream>::connect(addr).await
    }
}

#[allow(unsafe_code)]
#[cfg(feature = "async-io")]
unsafe impl<H: IO + Send + 'static> IoSafe for IOHandle<H> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyn_compat() {
        struct Test {
            _reactor: Box<dyn Reactor>,
        }

        let _ = Test {
            _reactor: Box::new(AsyncIO),
        };
    }
}
