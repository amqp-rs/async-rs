use async_trait::async_trait;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A wrapper around implementation-specific tasks that implement the TaskImpl trait
#[derive(Debug)]
pub struct Task<I: TaskImpl>(I);

impl<I: TaskImpl> Task<I> {
    /// Cancel the task, returning data if it was alredy finished
    pub async fn cancel(&mut self) -> Option<<Self as Future>::Output> {
        self.0.cancel().await
    }
}

impl<I: TaskImpl> From<I> for Task<I> {
    fn from(task_impl: I) -> Self {
        Self(task_impl)
    }
}

impl<I: TaskImpl> Future for Task<I> {
    type Output = <I as Future>::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

impl<I: TaskImpl> Drop for Task<I> {
    fn drop(&mut self) {
        self.0.detach();
    }
}

/// A common interface to wait for a Task completion, let it run n the background or cancel it.
#[async_trait]
pub trait TaskImpl: Future + Send + Unpin + 'static {
    /// Cancels the task and waits for it to stop running.
    ///
    /// Returns the task's output if it was completed just before it got canceled, or None if it
    /// didn't complete.
    async fn cancel(&mut self) -> Option<<Self as Future>::Output> {
        None
    }

    /// "Detach" the task from the current context to let it run in the background.
    ///
    /// Note that this is automatically called when dropping the Task so that it doesn't get
    /// canceled.
    fn detach(&mut self)
    where
        Self: Sized,
    {
    }
}
