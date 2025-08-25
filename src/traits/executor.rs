//! A collection of traits to define a common interface across executors

use crate::util::{Task, TaskImpl};
use std::{future::Future, ops::Deref};

/// A common interface for spawning futures on top of an executor
pub trait Executor {
    /// The type representing the tasks the are returned when spawning a Future on the executor
    type Task<T: Send + 'static>: TaskImpl<Output = T> + Send + 'static;

    /// Block on a future until completion
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T
    where
        Self: Sized;

    /// Spawn a future and return a handle to track its completion.
    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>>
    where
        Self: Sized;

    /// Convert a blocking task into a future, spawning it on a decicated thread pool
    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>>
    where
        Self: Sized;
}

impl<E: Deref> Executor for E
where
    E::Target: Executor + Sized,
{
    type Task<T: Send + 'static> = <E::Target as Executor>::Task<T>;

    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
        self.deref().block_on(f)
    }

    fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        self.deref().spawn(f)
    }

    fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        f: F,
    ) -> Task<Self::Task<T>> {
        self.deref().spawn_blocking(f)
    }
}
