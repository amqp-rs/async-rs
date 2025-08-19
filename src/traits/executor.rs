//! A collection of traits to define a common interface across executors

use async_trait::async_trait;
use std::{future::Future, ops::Deref};

/// A common interface for spawning futures on top of an executor
pub trait Executor {
    /// Block on a future until completion
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T
    where
        Self: Sized;

    /// Spawn a future and return a handle to track its completion.
    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static
    where
        Self: Sized;

    /// Convert a blocking task into a future, spawning it on a decicated thread pool
    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static
    where
        Self: Sized;
}

impl<E: Deref> Executor for E
where
    E::Target: Executor + Sized,
{
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        self.deref().block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        self.deref().spawn(f)
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> + 'static {
        self.deref().spawn_blocking(f)
    }
}

/// A common interface to wait for a Task completion, let it run n the background or cancel it.
#[async_trait]
pub trait Task<T: Send + 'static>: Future<Output = T> + Send + 'static {
    /// Cancels the task and waits for it to stop running.
    ///
    /// Returns the task's output if it was completed just before it got canceled, or None if it
    /// didn't complete.
    async fn cancel(&mut self) -> Option<T>;
}
