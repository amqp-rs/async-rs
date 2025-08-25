//! async-global-executor implementation of async runtime definition traits

use crate::{
    Runtime,
    traits::Executor,
    util::{RuntimeParts, Task},
};
use std::future::Future;

use task::AGETask;

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

impl Executor for AsyncGlobalExecutor {
    type Task<T: Send + 'static> = AGETask<T>;

    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        async_global_executor::block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        AGETask(Some(async_global_executor::spawn(f))).into()
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        AGETask(Some(async_global_executor::spawn_blocking(f))).into()
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

    /// An async-global-executor task
    #[derive(Debug)]
    pub struct AGETask<T: Send + 'static>(pub(super) Option<async_global_executor::Task<T>>);

    #[async_trait]
    impl<T: Send + 'static> TaskImpl for AGETask<T> {
        async fn cancel(&mut self) -> Option<T> {
            self.0.take()?.cancel().await
        }

        fn detach(&mut self) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_traits() {
        use crate::util::test::*;
        #[cfg(feature = "async-io")]
        let runtime = Runtime::async_global_executor();
        #[cfg(not(feature = "async-io"))]
        let runtime = AsyncGlobalExecutor;
        assert_send(&runtime);
        assert_sync(&runtime);
        assert_clone(&runtime);
    }
}
