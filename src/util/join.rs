use std::{
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Drive two futures concurrently, returning both outputs once both complete.
///
/// Both futures are polled on each wake; neither starves the other. This is a minimal,
/// runtime-agnostic equivalent of `futures::join!` for the two-future case. The futures are boxed so
/// the combinator needs no `unsafe` pin projection.
pub fn join<A: Future, B: Future>(a: A, b: B) -> Join<A, B> {
    Join {
        a: MaybeDone::new(a),
        b: MaybeDone::new(b),
    }
}

/// Drive two fallible futures concurrently, short-circuiting on the first error.
///
/// On the first `Err`, that error is returned without polling the other future again; otherwise
/// both `Ok` values are returned once both complete. Useful for "run these two halves until one of
/// them fails, then tear both down": the unfinished half is cancelled when the caller drops the
/// [`TryJoin`], which is what dropping this future on the spot amounts to.
pub fn try_join<T1, T2, E, A, B>(a: A, b: B) -> TryJoin<A, B>
where
    A: Future<Output = Result<T1, E>>,
    B: Future<Output = Result<T2, E>>,
{
    TryJoin {
        a: MaybeDone::new(a),
        b: MaybeDone::new(b),
    }
}

/// A boxed future that retains its output once it resolves.
struct MaybeDone<F: Future> {
    fut: Option<Pin<Box<F>>>,
    output: Option<F::Output>,
}

impl<F: Future> MaybeDone<F> {
    fn new(f: F) -> Self {
        Self {
            fut: Some(Box::pin(f)),
            output: None,
        }
    }

    /// Poll the inner future if still pending, stashing its output. Returns whether it is now done.
    fn poll(&mut self, cx: &mut Context<'_>) -> bool {
        if let Some(fut) = self.fut.as_mut()
            && let Poll::Ready(out) = fut.as_mut().poll(cx)
        {
            self.output = Some(out);
            self.fut = None;
        }
        self.fut.is_none()
    }

    fn is_done(&self) -> bool {
        self.fut.is_none()
    }

    fn take(&mut self) -> F::Output {
        self.output.take().expect("take on a not-yet-done future")
    }
}

impl<T, E, F: Future<Output = Result<T, E>>> MaybeDone<F> {
    /// Poll the inner future, taking its error out if that is what it completed with.
    fn poll_err(&mut self, cx: &mut Context<'_>) -> Option<E> {
        if !self.poll(cx) || !matches!(self.output, Some(Err(_))) {
            return None;
        }
        self.take().err()
    }
}

impl<F: Future> fmt::Debug for MaybeDone<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaybeDone")
            .field("done", &self.is_done())
            .finish()
    }
}

/// Future returned by [`join`].
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Join<A: Future, B: Future> {
    a: MaybeDone<A>,
    b: MaybeDone<B>,
}

// Written out rather than derived: a derive would ask for `A: Debug, B: Debug`, which the async
// blocks these are built from never satisfy.
impl<A: Future, B: Future> fmt::Debug for Join<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join")
            .field("a", &self.a)
            .field("b", &self.b)
            .finish()
    }
}

impl<A: Future, B: Future> Future for Join<A, B> {
    type Output = (A::Output, B::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `Self` is `Unpin` (the futures are boxed), so we can freely take a `&mut`.
        let this = self.get_mut();
        let a_done = this.a.poll(cx);
        let b_done = this.b.poll(cx);
        if a_done && b_done {
            Poll::Ready((this.a.take(), this.b.take()))
        } else {
            Poll::Pending
        }
    }
}

// Boxed futures make the combinators `Unpin` regardless of the inner futures.
impl<A: Future, B: Future> Unpin for Join<A, B> {}

/// Future returned by [`try_join`].
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct TryJoin<A: Future, B: Future> {
    a: MaybeDone<A>,
    b: MaybeDone<B>,
}

impl<A: Future, B: Future> fmt::Debug for TryJoin<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TryJoin")
            .field("a", &self.a)
            .field("b", &self.b)
            .finish()
    }
}

impl<A: Future, B: Future> Unpin for TryJoin<A, B> {}

impl<T1, T2, E, A, B> Future for TryJoin<A, B>
where
    A: Future<Output = Result<T1, E>>,
    B: Future<Output = Result<T2, E>>,
{
    type Output = Result<(T1, T2), E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Poll both, short-circuiting the moment one completes with an error (which drops — cancels
        // — the other when `this` is dropped by the caller).
        if let Some(err) = this.a.poll_err(cx) {
            return Poll::Ready(Err(err));
        }
        if let Some(err) = this.b.poll_err(cx) {
            return Poll::Ready(Err(err));
        }

        if this.a.is_done() && this.b.is_done() {
            let a = this.a.take().ok().expect("checked Ok");
            let b = this.b.take().ok().expect("checked Ok");
            Poll::Ready(Ok((a, b)))
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::simple_block_on;
    use std::{cell::Cell, future::poll_fn, rc::Rc};

    // A future that returns Pending `pendings` times before yielding `val`.
    fn delayed<T: Clone + 'static>(pendings: usize, val: T) -> impl Future<Output = T> {
        let left = Rc::new(Cell::new(pendings));
        poll_fn(move |cx: &mut Context<'_>| {
            if left.get() == 0 {
                Poll::Ready(val.clone())
            } else {
                left.set(left.get() - 1);
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
    }

    // The types must stay Debug for the futures they are actually built from, which a derived
    // impl would not manage: async blocks are not Debug.
    #[test]
    fn combinators_are_debug_over_async_blocks() {
        fn debug<T: fmt::Debug>(t: &T) -> String {
            format!("{t:?}")
        }
        debug(&join(async { 1u8 }, async { 2u8 }));
        debug(&try_join(async { Ok::<u8, ()>(1) }, async {
            Ok::<u8, ()>(2)
        }));
    }

    #[test]
    fn join_returns_both() {
        let (a, b) = simple_block_on(join(delayed(2, 1u8), delayed(5, "x")));
        assert_eq!(a, 1);
        assert_eq!(b, "x");
    }

    #[test]
    fn try_join_ok_returns_both() {
        let out: Result<(u8, u8), ()> =
            simple_block_on(try_join(delayed(1, Ok(1u8)), delayed(3, Ok(2u8))));
        assert_eq!(out, Ok((1, 2)));
    }

    #[test]
    fn try_join_short_circuits_on_error() {
        // The error future resolves first; the other never resolves on its own, so a successful
        // return proves try_join short-circuited and dropped (cancelled) it.
        let other = poll_fn(|cx: &mut Context<'_>| {
            cx.waker().wake_by_ref();
            Poll::<Result<u8, &str>>::Pending
        });
        let err = delayed(1, Err::<u8, &str>("boom"));
        let out = simple_block_on(try_join(err, other));
        assert_eq!(out, Err("boom"));
    }
}
