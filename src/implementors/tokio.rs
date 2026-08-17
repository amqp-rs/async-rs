//! tokio implementation of async runtime definition traits

use crate::{
    Runtime,
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit},
    util::Task,
};
use async_compat::{Compat, CompatExt};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    future::Future,
    io::{self, Read, Write},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::{
    net::TcpStream,
    runtime::{EnterGuard, Handle, Runtime as TokioRT},
    time::Sleep,
};
use tokio_stream::{StreamExt, wrappers::IntervalStream};

use task::TTask;

/// Type alias for the tokio runtime
pub type TokioRuntime = Runtime<Tokio>;

impl TokioRuntime {
    /// Create a new TokioRuntime backed by a freshly created tokio multi-threaded runtime.
    pub fn tokio() -> io::Result<Self> {
        Ok(Self::tokio_with_runtime(TokioRT::new()?))
    }

    /// Create a new TokioRuntime and bind it to the current tokio runtime by default.
    #[must_use]
    pub fn tokio_current() -> Self {
        Self::new(Tokio::current())
    }

    /// Create a new TokioRuntime and bind it to the tokio runtime associated to this handle by default.
    #[must_use]
    pub fn tokio_with_handle(handle: Handle) -> Self {
        Self::new(Tokio::default().with_handle(handle))
    }

    /// Create a new TokioRuntime and bind it to this tokio runtime.
    #[must_use]
    pub fn tokio_with_runtime(runtime: TokioRT) -> Self {
        Self::new(Tokio::default().with_runtime(runtime))
    }
}

/// What every entry point says when it cannot find a runtime to work with.
///
/// The ones returning an `io::Result` — `register` and `tcp_connect_addr` — report it, the rest
/// panic with it, but none of them may leave the caller with tokio's own "there is no reactor
/// running" thrown from somewhere further in, which says nothing about how to fix it.
const NO_RUNTIME: &str = "no tokio runtime: use Runtime::tokio() or Runtime::tokio_with_handle()";

/// The [`RuntimeKit`] implementation backed by the tokio async runtime
#[derive(Default, Clone, Debug)]
pub struct Tokio {
    handle: Option<Handle>,
    runtime: Option<Arc<TokioRT>>,
}

impl Tokio {
    /// Bind to the tokio Runtime associated to this handle by default.
    ///
    /// A runtime given to [`with_runtime`](Self::with_runtime) wins over this one whichever order
    /// the two are called in: only the owned runtime can be driven by
    /// [`block_on`](crate::traits::Executor::block_on), so letting a handle override it would bind
    /// half the kit to one runtime and half to another.
    #[must_use]
    pub fn with_handle(mut self, handle: Handle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Bind to this tokio runtime by default.
    #[must_use]
    pub fn with_runtime(mut self, runtime: TokioRT) -> Self {
        let handle = runtime.handle().clone();
        self.runtime = Some(Arc::new(runtime));
        self.with_handle(handle)
    }

    /// Bind to the current tokio Runtime by default.
    #[must_use]
    pub fn current() -> Self {
        Self::default().with_handle(Handle::current())
    }

    /// The runtime this kit is bound to, if any.
    ///
    /// Every entry point resolves through this so the kit cannot end up straddling two runtimes:
    /// `with_runtime` also records a handle, but `with_handle` may be called afterwards, and
    /// `block_on` can only drive the owned one.
    fn bound_handle(&self) -> Option<&Handle> {
        self.runtime
            .as_ref()
            .map(|r| r.handle())
            .or(self.handle.as_ref())
    }

    fn handle(&self) -> Option<Handle> {
        self.bound_handle()
            .cloned()
            .or_else(|| Handle::try_current().ok())
    }

    /// Enter the runtime this kit is bound to, if any.
    ///
    /// `None` is not a failure: an unbound kit runs on whichever runtime the caller is already in,
    /// and entering that one again would be a no-op. Whether there is one at all is a separate
    /// question, which [`has_runtime`](Self::has_runtime) answers.
    fn enter(&self) -> Option<EnterGuard<'_>> {
        self.bound_handle().map(Handle::enter)
    }

    /// Whether anything will be there to serve the call: our own runtime, or the caller's.
    fn has_runtime(&self) -> bool {
        self.bound_handle().is_some() || Handle::try_current().is_ok()
    }

    /// [`enter`](Self::enter), for the entry points which have nowhere to report a failure.
    ///
    /// `sleep` and `interval` capture their handle as they are constructed, so tokio would panic
    /// from inside them with a message which does not mention this crate. Fail with ours first.
    fn require_enter(&self) -> Option<EnterGuard<'_>> {
        assert!(self.has_runtime(), "{NO_RUNTIME}");
        self.enter()
    }

    fn require_handle(&self) -> Handle {
        self.handle().expect(NO_RUNTIME)
    }
}

impl RuntimeKit for Tokio {}

impl Executor for Tokio {
    type Task<T: Send + 'static> = TTask<T>;

    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        if let Some(runtime) = self.runtime.as_ref() {
            runtime.block_on(f)
        } else {
            // handle() already falls back to the ambient runtime, so there is nowhere left to
            // look once it comes back empty.
            self.require_handle().block_on(f)
        }
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        TTask(Some(self.require_handle().spawn(f))).into()
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        TTask(Some(self.require_handle().spawn_blocking(f))).into()
    }
}

impl Reactor for Tokio {
    type TcpStream = Compat<TcpStream>;
    type Sleep = Sleep;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        // AsyncFd::new reaches for the current runtime and panics when there is none. We return an
        // io::Result, so answer the question ourselves rather than letting it unwind from in there.
        if !self.has_runtime() {
            return Err(io::Error::other(NO_RUNTIME));
        }
        let _enter = self.enter();
        #[cfg(unix)]
        {
            Ok(unix::AsyncFdWrapper(tokio::io::unix::AsyncFd::new(socket)?))
        }
        #[cfg(not(unix))]
        {
            let _ = socket;
            Err::<crate::util::DummyIO, _>(io::Error::other(
                "Registering FD on tokio reactor is only supported on unix",
            ))
        }
    }

    fn sleep(&self, dur: Duration) -> Self::Sleep {
        let _enter = self.require_enter();
        tokio::time::sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        let _enter = self.require_enter();
        IntervalStream::new(tokio::time::interval(dur)).map(tokio::time::Instant::into_std)
    }

    fn tcp_connect_addr(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        // Unlike sleep and interval, which grab their handle as they are constructed, connecting
        // only touches the reactor once the future is polled, which can be from anywhere. Carry
        // the context along instead of entering it here, where it would be gone by then.
        //
        // Only the kit's own binding is resolved now, so the future binds to the kit's runtime
        // rather than to whichever one happens to poll it later. An unbound kit has nothing to
        // carry and falls back to the ambient runtime -- but at poll time, which is the only
        // moment there is one to find: deciding here would condemn a future built on a plain
        // thread even when it is later polled inside a perfectly good runtime.
        InTokioContext::new(self.bound_handle().cloned(), async move {
            // Our siblings panic outright when there is no runtime to be found, but this one
            // returns an io::Result, so say so properly instead of letting the caller trip over
            // tokio's own "there is no reactor running" panic from inside connect. Asked from in
            // here, the question is answered under whichever context InTokioContext just entered.
            if !crate::util::inside_tokio() {
                return Err(io::Error::other(NO_RUNTIME));
            }
            let stream = TcpStream::connect(addr).await?;
            stream.set_nodelay(true)?;
            Ok(stream.compat())
        })
    }
}

/// Drives a future inside a given tokio context, so it may be polled from a foreign executor.
///
/// The guard is taken and released within each `poll` rather than held across await points: an
/// `EnterGuard` is not `Send`, and keeping one in the future's state would make the whole future
/// `!Send`.
///
/// Only the handle is kept, deliberately: holding the kit's `Arc<TokioRT>` to keep the runtime
/// alive would let this future become its last owner, and dropping a tokio `Runtime` from an
/// async context panics outright, so a connect future dropped on a worker thread of that same
/// runtime would blow up far from the cause. Entering a runtime which has since shut down merely
/// fails the connect, which is the better of the two.
struct InTokioContext<F: Future> {
    handle: Option<Handle>,
    // Boxed to get a stable address without hand-rolling a pin projection, as util::join does.
    fut: Pin<Box<F>>,
}

impl<F: Future> InTokioContext<F> {
    fn new(handle: Option<Handle>, fut: F) -> Self {
        Self {
            handle,
            fut: Box::pin(fut),
        }
    }
}

impl<F: Future> Future for InTokioContext<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let _enter = this.handle.as_ref().map(Handle::enter);
        this.fut.as_mut().poll(cx)
    }
}

mod task {
    use crate::util::TaskImpl;
    use async_trait::async_trait;
    use std::{
        future::Future,
        panic,
        pin::Pin,
        task::{Context, Poll},
    };

    /// A tokio task
    #[derive(Debug)]
    pub struct TTask<T: Send + 'static>(pub(super) Option<tokio::task::JoinHandle<T>>);

    #[async_trait]
    impl<T: Send + 'static> TaskImpl for TTask<T> {
        async fn cancel(&mut self) -> Option<T> {
            let task = self.0.take()?;
            task.abort();
            task.await.ok()
        }
    }

    impl<T: Send + 'static> Future for TTask<T> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let task = self
                .0
                .as_mut()
                .expect("Task polled after it was canceled or completed");
            let res = match Pin::new(task).poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(res) => res,
            };

            // Drop the handle now that it has completed: polling it again would trip tokio's own
            // "JoinHandle polled after completion" assertion.
            self.0 = None;

            match res {
                Ok(res) => Poll::Ready(res),
                // Our Output is T, so a failed task has no value to yield. Report it the way
                // async-task (and thus the smol and async-global-executor backends) already does
                // rather than stalling forever on a Pending nobody will ever wake.
                Err(err) if err.is_panic() => panic::resume_unwind(err.into_panic()),
                Err(err) => panic!("Task did not complete: {err}"),
            }
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use futures_io::{AsyncRead, AsyncWrite};
    use std::{
        io::{IoSlice, IoSliceMut},
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::io::unix::AsyncFd;

    pub(super) struct AsyncFdWrapper<H: Read + Write + AsSysFd>(pub(super) AsyncFd<H>);

    impl<H: Read + Write + AsSysFd> AsyncFdWrapper<H> {
        fn read<F: FnOnce(&mut AsyncFd<H>) -> io::Result<usize>>(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            f: F,
        ) -> Option<Poll<io::Result<usize>>> {
            Some(match self.0.poll_read_ready_mut(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Ready(Ok(mut guard)) => match guard.try_io(f) {
                    Ok(res) => Poll::Ready(res),
                    Err(_) => return None,
                },
            })
        }

        fn write<R, F: FnOnce(&mut AsyncFd<H>) -> io::Result<R>>(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            f: F,
        ) -> Option<Poll<io::Result<R>>> {
            Some(match self.0.poll_write_ready_mut(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Ready(Ok(mut guard)) => match guard.try_io(f) {
                    Ok(res) => Poll::Ready(res),
                    Err(_) => return None,
                },
            })
        }
    }

    impl<H: Read + Write + AsSysFd> Unpin for AsyncFdWrapper<H> {}

    impl<H: Read + Write + AsSysFd> AsyncRead for AsyncFdWrapper<H> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            loop {
                if let Some(res) = self.as_mut().read(cx, |socket| socket.get_mut().read(buf)) {
                    return res;
                }
            }
        }

        fn poll_read_vectored(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &mut [IoSliceMut<'_>],
        ) -> Poll<io::Result<usize>> {
            loop {
                if let Some(res) = self
                    .as_mut()
                    .read(cx, |socket| socket.get_mut().read_vectored(bufs))
                {
                    return res;
                }
            }
        }
    }

    impl<H: Read + Write + AsSysFd> AsyncWrite for AsyncFdWrapper<H> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            loop {
                if let Some(res) = self
                    .as_mut()
                    .write(cx, |socket| socket.get_mut().write(buf))
                {
                    return res;
                }
            }
        }

        fn poll_write_vectored(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<io::Result<usize>> {
            loop {
                if let Some(res) = self
                    .as_mut()
                    .write(cx, |socket| socket.get_mut().write_vectored(bufs))
                {
                    return res;
                }
            }
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            loop {
                if let Some(res) = self.as_mut().write(cx, |socket| socket.get_mut().flush()) {
                    return res;
                }
            }
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<futures_io::Result<()>> {
            self.poll_flush(cx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_traits() {
        use crate::util::test::*;
        let runtime = Runtime::tokio().unwrap();
        assert_send(&runtime);
        assert_sync(&runtime);
        assert_clone(&runtime);
    }

    // A failed task used to resolve to a Pending nobody would ever wake, hanging the caller
    // forever. Both of these must now come back, panicking, in bounded time.
    #[test]
    fn panicking_task_does_not_hang() {
        let res = crate::util::test::with_timeout(|| {
            let runtime = Runtime::tokio().unwrap();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(runtime.spawn(async { panic!("boom") }))
            }))
        });
        // Down to the payload: asserting only that something panicked would also pass if the
        // panic were a fresh one of our own rather than the task's, resumed.
        assert_eq!(
            res.expect_err("task panic").downcast_ref::<&str>(),
            Some(&"boom")
        );
    }

    // The returned future must carry its tokio context with it: RuntimeParts pairs this reactor
    // with a foreign executor, which polls it with no tokio runtime in scope.
    #[test]
    fn tcp_connect_addr_polled_off_runtime() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // The runtime has to outlive the connect: the stream it hands back stays registered on it.
        let (_runtime, mut stream) = crate::util::test::with_timeout(move || {
            let runtime = Runtime::tokio().unwrap();
            let connect = runtime.tcp_connect_addr(addr);
            let stream = crate::util::simple_block_on(connect).expect("connect");
            (runtime, stream)
        });

        // The listener never leaves this thread, so it is released however the test ends. Handing
        // it to a helper to accept on would strand that helper on the very failure we guard here,
        // holding its port for the rest of the binary.
        let (mut socket, _) = listener.accept().expect("accept");
        Write::write_all(&mut socket, b"hello").expect("write");

        // Connecting is only half the property. The per-poll EnterGuard is long gone by now, and
        // the stream we were handed still has to be usable off the runtime -- that is what makes
        // entering per poll, rather than holding a guard across awaits, a safe design.
        let read = crate::util::test::with_timeout(move || {
            let mut buf = [0_u8; 5];
            let mut read = 0;
            crate::util::simple_block_on(std::future::poll_fn(|cx| {
                while read < buf.len() {
                    match Pin::new(&mut stream).poll_read(cx, &mut buf[read..]) {
                        Poll::Ready(Ok(0)) => break,
                        Poll::Ready(Ok(n)) => read += n,
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                Poll::Ready(Ok(buf))
            }))
            .expect("read")
        });
        assert_eq!(&read, b"hello");
    }

    // with_runtime records a handle too, so a kit handed each in turn used to resolve connect
    // through one runtime and its timers and registrations through the other.
    #[test]
    fn one_kit_binds_everything_to_the_same_runtime() {
        let other = TokioRT::new().unwrap();
        let runtime = Runtime::new(
            Tokio::default()
                .with_runtime(TokioRT::new().unwrap())
                .with_handle(other.handle().clone()),
        );
        // Nothing may be left pointing at `other` once it is gone.
        drop(other);

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || listener.accept().map(|_| ()));
        runtime.block_on(async { runtime.tcp_connect_addr(addr).await.expect("connect") });
        accepted.join().expect("accept thread").expect("accept");
    }

    // The connect path returns an io::Result, so a missing runtime is reportable rather than a
    // panic thrown from inside tokio once someone gets around to polling the future.
    #[test]
    fn tcp_connect_addr_without_a_runtime_reports_an_error() {
        let runtime = Runtime::new(Tokio::default());
        let addr = "127.0.0.1:1".parse().unwrap();
        let Err(err) = crate::util::simple_block_on(runtime.tcp_connect_addr(addr)) else {
            panic!("connect succeeded without a runtime");
        };
        assert!(err.to_string().contains("no tokio runtime"), "{err}");
    }

    // The mirror image of the test above: an unbound kit has no runtime of its own to carry, so
    // the ambient one has to be looked for when the future is polled rather than when it is
    // built. Resolving it eagerly condemns this future on the spot, on a thread which never had a
    // runtime to offer, even though the one polling it does.
    #[test]
    fn tcp_connect_addr_built_off_runtime_uses_the_one_polling_it() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || listener.accept().map(|_| ()));

        // Built here, with nothing in scope, and polled by a runtime it knows nothing about.
        let connect = Runtime::new(Tokio::default()).tcp_connect_addr(addr);
        TokioRT::new()
            .unwrap()
            .block_on(connect)
            .expect("connect polled inside a runtime");
        accepted.join().expect("accept thread").expect("accept");
    }

    // register hands back an io::Result too, so it owes the caller the same answer connect gives
    // rather than tokio's panic from inside AsyncFd::new.
    #[test]
    #[cfg(unix)]
    fn register_without_a_runtime_reports_an_error() {
        let runtime = Runtime::new(Tokio::default());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let socket = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let Err(err) = runtime.register(socket) else {
            panic!("register succeeded without a runtime");
        };
        assert!(err.to_string().contains("no tokio runtime"), "{err}");
    }

    #[test]
    fn panicking_blocking_task_does_not_hang() {
        let res = crate::util::test::with_timeout(|| {
            let runtime = Runtime::tokio().unwrap();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(runtime.spawn_blocking(|| -> u32 { panic!("boom") }))
            }))
        });
        assert_eq!(
            res.expect_err("task panic").downcast_ref::<&str>(),
            Some(&"boom")
        );
    }
}
