use crate::{
    sys::AsSysFd,
    traits::Reactor,
    util::{IOHandle, UnitFuture},
};
use async_io::{Async, Timer};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    future::Future,
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

/// Dummy object implementing reactor common interfaces on top of async-io
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncIO;

impl Reactor for AsyncIO {
    type TcpStream = Async<TcpStream>;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        Async::new(IOHandle::new(socket))
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> + Send + 'static {
        UnitFuture(Timer::after(dur))
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        Timer::interval(dur)
    }

    fn tcp_connect(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        Async::<TcpStream>::connect(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyn_compat() {
        struct Test {
            _reactor: Box<dyn Reactor<TcpStream = Async<TcpStream>>>,
        }

        let _ = Test {
            _reactor: Box::new(AsyncIO),
        };
    }
}
