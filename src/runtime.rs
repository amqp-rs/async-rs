use crate::{Executor, Reactor, RuntimeKit, Task};
use std::{fmt::Debug, future::Future, pin::Pin};

/// A full-featured Runtime implementation
#[derive(Debug)]
pub struct Runtime<RK: RuntimeKit + 'static> {
    kit: RK,
}

impl<RK: RuntimeKit + 'static> Runtime<RK> {
    /// Create a new Runtime from a RuntimeKit
    pub fn new(kit: RK) -> Self {
        Self { kit }
    }
}

impl<RK: RuntimeKit + 'static> From<RK> for Runtime<RK> {
    fn from(kit: RK) -> Self {
        Self::new(kit)
    }
}

impl<RK: RuntimeKit + 'static> Executor for Runtime<RK> {
    fn block_on<T>(&self, f: Pin<Box<dyn Future<Output = T>>>) -> T {
        self.kit.block_on(f)
    }

    fn spawn<T: Send + 'static>(
        &self,
        f: impl Future<Output = T> + Send + 'static,
    ) -> impl Task<T> {
        self.kit.spawn(f)
    }

    fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Task<T> {
        self.kit.spawn_blocking(f)
    }
}

/// Wrapper around separate Executor and Reactor implementing RuntimeKit
#[derive(Debug)]
pub struct RuntimeParts<E: Executor, R: Reactor> {
    executor: E,
    _reactor: R,
}

impl<E: Executor, R: Reactor> RuntimeParts<E, R> {
    /// Create new RuntimeParts from separate Executor and Reactor
    pub fn new(executor: E, reactor: R) -> Self {
        Self {
            executor,
            _reactor: reactor,
        }
    }
}

impl<E: Executor, R: Reactor> Executor for RuntimeParts<E, R> {
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

impl<E: Executor, R: Reactor> RuntimeKit for RuntimeParts<E, R> {}
