use crate::{Executor, RuntimeKit, Task};
use alloc::{boxed::Box, fmt::Debug};
use core::{future::Future, marker::PhantomData, pin::Pin};

/// Wrapper around separate Executor and Reactor implementing RuntimeKit
#[derive(Debug)]
pub struct RuntimeParts<E: Executor, R: Debug /* TODO: Reactor */> {
    executor: E,
    _reactor: PhantomData<R>,
}

impl<E: Executor, R: Debug> RuntimeParts<E, R> {
    /// Create new RuntimeParts from separate Executor and Reactor
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            _reactor: PhantomData {},
        }
    }
}

impl<E: Executor, R: Debug> Executor for RuntimeParts<E, R> {
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T {
        self.executor.block_on(f)
    }

    fn spawn<T: Send + 'static>(
        &self,
        f: impl Future<Output = T> + Send + 'static,
    ) -> impl Task<T> {
        self.executor.spawn(f)
    }

    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> {
        self.executor.spawn_blocking(f)
    }
}

impl<E: Executor, R: Debug> RuntimeKit for RuntimeParts<E, R> {}
