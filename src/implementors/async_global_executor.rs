//! async-global-executor implementation of async runtime definition traits

use crate::{
    Runtime,
    traits::{Executor, Task},
    util::RuntimeParts,
};
use async_trait::async_trait;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "async-io")]
use crate::AsyncIO;

/// Type alias for the async-global-executor runtime
#[cfg(feature = "async-io")]
pub type AGERuntime = Runtime<RuntimeParts<AsyncGlobalExecutor, AsyncIO>>;

#[cfg(feature = "async-io")]
impl AGERuntime {
    /// Create a new SmolRuntime
    pub fn async_global_executor() -> Self {
        Self::new(RuntimeParts::new(AsyncGlobalExecutor, AsyncIO))
    }
}

/// Dummy object implementing executor common interfaces on top of async-global-executor
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncGlobalExecutor;

struct AGETask<T: Send + 'static>(Option<async_global_executor::Task<T>>);

impl Executor for AsyncGlobalExecutor {
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        async_global_executor::block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        AGETask(Some(async_global_executor::spawn(f)))
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        AGETask(Some(async_global_executor::spawn_blocking(f)))
    }
}

#[async_trait]
impl<T: Send + 'static> Task<T> for AGETask<T> {
    async fn cancel(&mut self) -> Option<T> {
        self.0.take()?.cancel().await
    }
}

impl<T: Send + 'static> Drop for AGETask<T> {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.detach();
        }
    }
}

impl<T: Send + 'static> Future for AGETask<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(self.0.as_mut().expect("task canceled")).poll(cx)
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
            _executor: Box::new(AsyncGlobalExecutor),
            _task: Box::new(AGETask(None)),
        };
    }
}
