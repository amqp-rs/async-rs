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
    #[must_use]
    pub fn noop() -> Self {
        Self::new(Noop)
    }
}

/// A no-op [`RuntimeKit`] implementation that never actually executes tasks or I/O
///
/// `spawn` and `spawn_blocking` drop the work they are handed and return a task which never
/// completes, so anything built on them never resolves either: awaiting the result of
/// [`Runtime::to_socket_addrs`](crate::Runtime::to_socket_addrs) on a `NoopRuntime` waits forever,
/// and parked rather than spinning, since that is what
/// [`simple_block_on`](crate::util::simple_block_on) does with a future which never wakes.
///
/// The [`Reactor`] side resolves under any executor, not just [`Executor::block_on`], but only as
/// far as handing something back: `sleep` completes immediately, and `tcp_connect_addr` hands over
/// a [`DummyIO`](crate::util::DummyIO) without having connected to anything. Using it is where the
/// waiting starts again — every read, write, flush and close on a `DummyIO`, and every item of the
/// stream `interval` returns, is `Poll::Pending` forever, and no waker is ever registered, so the
/// executor cannot even be woken to cancel the task waiting on one.
///
/// `register` is the one to watch: it takes the socket by value and drops it, closing the
/// descriptor, and hands back a `DummyIO` in its place. Dropping a `NoopRuntime` into a test
/// harness therefore loses the socket, silently.
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
