//! smol implementation of async runtime definition traits

use crate::{Executor, Runtime, RuntimeKit, Task};
use async_trait::async_trait;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
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

struct STask<T>(Option<smol::Task<T>>);

impl Executor for Smol {
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T {
        smol::block_on(f)
    }

    fn spawn<T: Send + 'static>(
        &self,
        f: impl Future<Output = T> + Send + 'static,
    ) -> impl Task<T> {
        STask(Some(smol::spawn(f)))
    }

    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> {
        STask(Some(smol::unblock(f)))
    }
}

impl RuntimeKit for Smol {}

#[async_trait(?Send)]
impl<T> Task<T> for STask<T> {
    async fn cancel(&mut self) -> Option<T> {
        self.0.take()?.cancel().await
    }
}

impl<T> Drop for STask<T> {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.detach();
        }
    }
}

impl<T> Future for STask<T> {
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
            _executor: Box::new(Smol),
            _task: Box::new(STask(None)),
        };
    }
}
