use crate::{
    sys::IO,
    traits::{AsyncIOHandle, Reactor},
    util::{IOHandle, UnitFuture},
};
use async_io::{Async, Timer};
use futures_core::Stream;
use std::{
    future::Future,
    io,
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

/// Dummy object implementing reactor common interfaces on top of async-io
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncIO;

impl Reactor for AsyncIO {
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send> {
        Async::new(socket)
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> {
        UnitFuture(Timer::after(dur))
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> {
        Timer::interval(dur)
    }

    fn tcp_connect(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<impl AsyncIOHandle + Send>> + Send {
        Async::<TcpStream>::connect(addr)
    }
}

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
