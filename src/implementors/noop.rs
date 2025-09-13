//! noop implementation of async runtime definition traits

use crate::{
    Runtime,
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit},
    util::{self, DummyIO, DummyStream, Task},
};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    future::{self, Future, Ready},
    io::{self, Read, Write},
    marker::PhantomData,
    net::SocketAddr,
    time::{Duration, Instant},
};

use task::NTask;

/// Type alias for the noop runtime
pub type NoopRuntime = Runtime<Noop>;

impl NoopRuntime {
    /// Create a new NoopRuntime
    pub fn noop() -> Self {
        Self::new(Noop)
    }
}

/// Dummy object implementing async common interfaces on top of smol
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Noop;

impl RuntimeKit for Noop {}

impl Executor for Noop {
    type Task<T: Send + 'static> = NTask<T>;

    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        // We cannot fake something unless we require T: Default, which we don't want.
        // Let's get a minimalist implementation for this one.
        util::simple_block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        _f: F,
    ) -> Task<Self::Task<T>> {
        NTask(PhantomData).into()
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        _f: F,
    ) -> Task<Self::Task<T>> {
        NTask(PhantomData).into()
    }
}

impl Reactor for Noop {
    type TcpStream = DummyIO;
    type Sleep = Ready<()>;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        _socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        Ok(DummyIO)
    }

    fn sleep(&self, _dur: Duration) -> Self::Sleep {
        future::ready(())
    }

    fn interval(&self, _dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        DummyStream(PhantomData)
    }

    fn tcp_connect_addr(
        &self,
        _addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        async { Ok(DummyIO) }
    }
}

mod task {
    use crate::util::TaskImpl;
    use async_trait::async_trait;
    use std::{
        future::Future,
        marker::PhantomData,
        pin::Pin,
        task::{Context, Poll},
    };

    /// A noop task
    #[derive(Debug)]
    pub struct NTask<T: Send + 'static>(pub(super) PhantomData<T>);

    impl<T: Send + 'static> Unpin for NTask<T> {}

    #[async_trait]
    impl<T: Send + 'static> TaskImpl for NTask<T> {}

    impl<T: Send + 'static> Future for NTask<T> {
        type Output = T;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_traits() {
        use crate::util::test::*;
        let runtime = Runtime::noop();
        assert_send(&runtime);
        assert_sync(&runtime);
        assert_clone(&runtime);
    }
}
