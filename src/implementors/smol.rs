//! smol implementation of async runtime definition traits

use crate::{
    Runtime,
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit},
    util::{IOHandle, Task},
};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use smol::{Async, Timer};
use std::{
    future::Future,
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

use task::STask;

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

impl RuntimeKit for Smol {}

impl Executor for Smol {
    type Task<T: Send + 'static> = STask<T>;

    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        smol::block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        STask(Some(smol::spawn(f))).into()
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        STask(Some(smol::unblock(f))).into()
    }
}

impl Reactor for Smol {
    type TcpStream = Async<TcpStream>;
    type Sleep = Timer;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        Async::new(IOHandle::new(socket))
    }

    fn sleep(&self, dur: Duration) -> Self::Sleep {
        Timer::after(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        Timer::interval(dur)
    }

    fn tcp_connect_addr(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        Async::<TcpStream>::connect(addr)
    }
}

mod task {
    use crate::util::TaskImpl;
    use async_trait::async_trait;
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    /// A smol task
    #[derive(Debug)]
    pub struct STask<T: Send + 'static>(pub(super) Option<smol::Task<T>>);

    #[async_trait]
    impl<T: Send + 'static> TaskImpl for STask<T> {
        async fn cancel(&mut self) -> Option<T> {
            self.0.take()?.cancel().await
        }

        fn detach(&mut self) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_traits() {
        use crate::util::test::*;
        let runtime = Runtime::smol();
        assert_send(&runtime);
        assert_sync(&runtime);
        assert_clone(&runtime);
    }
}
