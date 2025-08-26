//! tokio implementation of async runtime definition traits

use crate::{
    Runtime,
    sys::AsSysFd,
    traits::{Executor, Reactor, RuntimeKit},
    util::Task,
};
use async_compat::{Compat, CompatExt};
use cfg_if::cfg_if;
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
};
use tokio_stream::{StreamExt, wrappers::IntervalStream};

use task::TTask;

/// Type alias for the tokio runtime
pub type TokioRuntime = Runtime<Tokio>;

impl TokioRuntime {
    /// Create a new TokioRuntime and bind it to this tokio runtime.
    pub fn tokio() -> io::Result<Self> {
        Ok(Self::tokio_with_runtime(TokioRT::new()?))
    }

    /// Create a new TokioRuntime and bind it to the current tokio runtime by default.
    pub fn tokio_current() -> Self {
        Self::new(Tokio::current())
    }

    /// Create a new TokioRuntime and bind it to the tokio runtime associated to this handle by default.
    pub fn tokio_with_handle(handle: Handle) -> Self {
        Self::new(Tokio::default().with_handle(handle))
    }

    /// Create a new TokioRuntime and bind it to this tokio runtime.
    pub fn tokio_with_runtime(runtime: TokioRT) -> Self {
        Self::new(Tokio::default().with_runtime(runtime))
    }
}

/// Dummy object implementing async common interfaces on top of tokio
#[derive(Default, Clone, Debug)]
pub struct Tokio {
    handle: Option<Handle>,
    runtime: Option<Arc<TokioRT>>,
}

impl Tokio {
    /// Bind to the tokio Runtime associated to this handle by default.
    pub fn with_handle(mut self, handle: Handle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Bind to the tokio Runtime associated to this handle by default.
    pub fn with_runtime(mut self, runtime: TokioRT) -> Self {
        let handle = runtime.handle().clone();
        self.runtime = Some(Arc::new(runtime));
        self.with_handle(handle)
    }

    /// Bind to the current tokio Runtime by default.
    pub fn current() -> Self {
        Self::default().with_handle(Handle::current())
    }

    fn handle(&self) -> Option<Handle> {
        self.runtime
            .as_ref()
            .map(|r| r.handle().clone())
            .or_else(|| Handle::try_current().ok())
            .or_else(|| self.handle.clone())
    }

    fn enter(&self) -> Option<EnterGuard<'_>> {
        self.runtime
            .as_ref()
            .map(|r| r.handle())
            .or(self.handle.as_ref())
            .map(Handle::enter)
    }
}

impl RuntimeKit for Tokio {}

impl Executor for Tokio {
    type Task<T: Send + 'static> = TTask<T>;

    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        if let Some(runtime) = self.runtime.as_ref() {
            runtime.block_on(f)
        } else if let Some(handle) = self.handle() {
            handle.block_on(f)
        } else {
            Handle::current().block_on(f)
        }
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        TTask(Some(if let Some(handle) = self.handle() {
            handle.spawn(f)
        } else {
            tokio::task::spawn(f)
        }))
        .into()
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        TTask(Some(if let Some(handle) = self.handle() {
            handle.spawn_blocking(f)
        } else {
            tokio::task::spawn_blocking(f)
        }))
        .into()
    }
}

impl Reactor for Tokio {
    type TcpStream = Compat<TcpStream>;

    fn register<H: Read + Write + AsSysFd + Send + 'static>(
        &self,
        socket: H,
    ) -> io::Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static> {
        let _enter = self.enter();
        cfg_if! {
            if #[cfg(unix)] {
                Ok(unix::AsyncFdWrapper(
                    tokio::io::unix::AsyncFd::new(socket)?,
                ))
            } else {
                Err::<crate::util::DummyIO, _>(io::Error::other(
                    "Registering FD on tokio reactor is only supported on unix",
                ))
            }
        }
    }

    fn sleep(&self, dur: Duration) -> impl Future<Output = ()> + Send + 'static {
        let _enter = self.enter();
        tokio::time::sleep(dur)
    }

    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant> + Send + 'static {
        let _enter = self.enter();
        IntervalStream::new(tokio::time::interval(dur)).map(tokio::time::Instant::into_std)
    }

    fn tcp_connect_addr(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send + 'static {
        let _enter = self.enter();
        async move { Ok(TcpStream::connect(addr).await?.compat()) }
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
            let task = self.0.as_mut().expect("task has been canceled");
            match Pin::new(task).poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(res) => Poll::Ready(res.expect("task has been canceled")),
            }
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use futures_io::{AsyncRead, AsyncWrite};
    use std::io::{IoSlice, IoSliceMut};
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

    impl<H: Read + Write + AsSysFd + Send + 'static> AsyncWrite for AsyncFdWrapper<H> {
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
}
