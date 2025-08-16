//! A collection of traits to define a common interface across executors

use alloc::boxed::Box;
use async_trait::async_trait;
use core::{future::Future, ops::Deref, pin::Pin};

/// A common interface for spawning futures on top of an executor
pub trait Executor {
    /// Block on a future until completion
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T
    where
        Self: Sized;

    /// Spawn a future and return a handle to track its completion.
    fn spawn<T: Send + 'static>(&self, f: impl Future<Output = T> + Send + 'static) -> impl Task<T>
    where
        Self: Sized;

    /// Convert a blocking task into a future, spawning it on a decicated thread pool
    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T>
    where
        Self: Sized;
}

impl<E: Deref + Sync> Executor for E
where
    Self: Sized,
    E::Target: Executor + Sync + Sized,
{
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T {
        self.deref().block_on(f)
    }

    fn spawn<T: Send + 'static>(
        &self,
        f: impl Future<Output = T> + Send + 'static,
    ) -> impl Task<T> {
        self.deref().spawn(f)
    }

    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> {
        self.deref().spawn_blocking(f)
    }
}

/// A common interface to wait for a Task completion, let it run n the background or cancel it.
#[async_trait(?Send)]
pub trait Task<T>: Future<Output = T> {
    /// Cancels the task and waits for it to stop running.
    ///
    /// Returns the task's output if it was completed just before it got canceled, or None if it
    /// didn't complete.
    async fn cancel(&mut self) -> Option<T>;
}
