use std::{
    fmt,
    future::Future,
    pin::Pin,
    task::{self, Context, Poll},
};

/// Wrap a Future to discard its output
pub struct UnitFuture<F: Future + Unpin>(pub F);

impl<F: Future + Unpin> Future for UnitFuture<F> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        task::ready!(Pin::new(&mut self.0).poll(cx));
        Poll::Ready(())
    }
}

impl<F: Future + Unpin> fmt::Debug for UnitFuture<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UnitFuture").finish()
    }
}
