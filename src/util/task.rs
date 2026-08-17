use async_trait::async_trait;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A wrapper around implementation-specific tasks that implement the TaskImpl trait
///
/// Awaiting a `Task` yields the task's output. That output type leaves no room to report a
/// failure, so on the backends which can detect one — tokio, smol and async-global-executor — a
/// task which panicked resumes its panic in the awaiting task, and awaiting one which was
/// canceled, or whose runtime went away, panics too. [`Noop`](crate::Noop) is the exception: it
/// runs nothing, and its tasks simply never complete.
///
/// Note that `cancel` takes `&mut self`, so awaiting a `Task` after canceling it is expressible
/// and panics rather than failing to compile. The same goes for a `cancel` which was itself
/// dropped before it completed — in a `select!`, or a [`TryJoin`](crate::util::TryJoin) which
/// short-circuited: it has already given up the underlying task by then, so the `Task` is spent
/// even though no cancellation result was ever handed back.
#[derive(Debug)]
pub struct Task<I: TaskImpl>(I);

impl<I: TaskImpl> Task<I> {
    /// Cancel the task, returning data if it was already finished
    ///
    /// This gives up the underlying task, so the `Task` has nothing left to wait for: see the
    /// type-level docs for what awaiting it afterwards does.
    ///
    /// `None` means the task did not complete, and does not say why: one which panicked comes back
    /// as `None` here rather than resuming its panic the way awaiting it would. The output type
    /// has no room to tell the two apart, on any of the backends.
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

/// A common interface to wait for a Task completion, let it run in the background or cancel it.
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
