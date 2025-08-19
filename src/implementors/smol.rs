//! smol implementation of async runtime definition traits

use crate::{
    Runtime,
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit, Task},
    util::{IOHandle, UnitFuture},
};
use async_trait::async_trait;
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use smol::{Async, Timer};
use std::{
    future::Future,
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// Type alias for the smol runtime
pub type SmolRuntime = Runtime<Smol>;

impl SmolRuntime {
    /// Create a new SmolRuntime
    pub fn smol() -> Self {
        Self::new(Smol)
    }
}

/// Dummy object implementing async common interfaces on top of smol
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Smol;

struct STask<T: Send + 'static>(Option<smol::Task<T>>);

impl RuntimeKit for Smol {}

impl Executor for Smol {
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        smol::block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        STask(Some(smol::spawn(f)))
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        STask(Some(smol::unblock(f)))
    }
}

#[async_trait]
impl<T: Send + 'static> Task<T> for STask<T> {
    async fn cancel(&mut self) -> Option<T> {
        self.0.take()?.cancel().await
    }
}

impl<T: Send + 'static> Drop for STask<T> {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.detach();
        }
    }
}

impl<T: Send + 'static> Future for STask<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(self.0.as_mut().expect("task canceled")).poll(cx)
    }
}

impl Reactor for Smol {
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
            _executor: Box<dyn Executor>,
            _reactor: Box<dyn Reactor<TcpStream = Async<TcpStream>>>,
            _kit: Box<dyn RuntimeKit<TcpStream = Async<TcpStream>>>,
            _task: Box<dyn Task<String>>,
        }

        let _ = Test {
            _executor: Box::new(Smol),
            _reactor: Box::new(Smol),
            _kit: Box::new(Smol),
            _task: Box::new(STask(None)),
        };
    }
}
