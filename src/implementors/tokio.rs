//! tokio implementation of async runtime definition traits

use crate::{Executor, Runtime, RuntimeKit, Task};
use async_trait::async_trait;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::runtime::Handle;

/// Type alias for the tokio runtime
pub type TokioRuntime = Runtime<Tokio>;

impl TokioRuntime {
    /// Create a new TokioRuntime and bind it to the current tokio runtime by default.
    pub fn tokio() -> Self {
        Self::new(Tokio::current())
    }

    /// Create a new TokioRuntime and bind it to the tokio runtime associated to this handle by default.
    pub fn tokio_with_handle(handle: Handle) -> Self {
        Self::new(Tokio::default().with_handle(handle))
    }
}

/// Dummy object implementing async common interfaces on top of tokio
#[derive(Default, Debug, Clone)]
pub struct Tokio {
    handle: Option<Handle>,
}

impl Tokio {
    /// Bind to the tokio Runtime associated to this handle by default.
    pub fn with_handle(mut self, handle: Handle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Bind to the current tokio Runtime by default.
    pub fn current() -> Self {
        Self::default().with_handle(Handle::current())
    }

    pub(crate) fn handle(&self) -> Option<Handle> {
        Handle::try_current().ok().or_else(|| self.handle.clone())
    }
}

struct TTask<T>(Option<tokio::task::JoinHandle<T>>);

impl Executor for Tokio {
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T {
        if let Some(handle) = self.handle() {
            handle.block_on(f)
        } else {
            Handle::current().block_on(f)
        }
    }

    fn spawn<T: Send + 'static>(
        &self,
        f: impl Future<Output = T> + Send + 'static,
    ) -> impl Task<T> {
        TTask(Some(if let Some(handle) = self.handle() {
            handle.spawn(f)
        } else {
            tokio::task::spawn(f)
        }))
    }

    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> {
        TTask(Some(if let Some(handle) = self.handle() {
            handle.spawn_blocking(f)
        } else {
            tokio::task::spawn_blocking(f)
        }))
    }
}

impl RuntimeKit for Tokio {}

#[async_trait(?Send)]
impl<T> Task<T> for TTask<T> {
    async fn cancel(&mut self) -> Option<T> {
        let task = self.0.take()?;
        task.abort();
        task.await.ok()
    }
}

impl<T> Future for TTask<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let task = self.0.as_mut().expect("task has been canceled");
        match Pin::new(task).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => Poll::Ready(res.expect("task has been canceled")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyn_compat() {
        struct Test {
            _executor: Box<dyn Executor>,
            _task: Box<dyn Task<String>>,
        }

        let _ = Test {
            _executor: Box::new(Tokio::default()),
            _task: Box::new(TTask(None)),
        };
    }
}
