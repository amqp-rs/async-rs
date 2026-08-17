use std::{
    cell::Cell,
    future::Future,
    pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake},
    thread::{self, Thread},
};

/// Simple naive block_on implementation for noop runtime
///
/// # Panics
///
/// Panics if called recursively, that is from within a future which is itself being driven by
/// `simple_block_on` on the same thread.
pub fn simple_block_on<F: Future>(f: F) -> F::Output {
    let _enter = enter();
    let thread = ThreadWaker::new_arc();
    let waker = thread.clone().into();
    let mut cx = Context::from_waker(&waker);
    let mut f = pin::pin!(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(r) => return r,
            Poll::Pending => thread.park(),
        }
    }
}

thread_local! {
    // Only ever touched by its own thread, no atomics required.
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

struct EnterGuard;

impl Drop for EnterGuard {
    fn drop(&mut self) {
        BUSY.set(false);
    }
}

fn enter() -> EnterGuard {
    if BUSY.replace(true) {
        panic!("Cannot call simple_block_on recursively")
    }

    EnterGuard
}

struct ThreadWaker {
    thread: Thread,
    /// `true` from the moment a wakeup is signalled until `park` consumes it.
    ///
    /// Set by `unpark` so that a wakeup happening between two `park` calls cannot be forgotten,
    /// which matters because the code we drive may park/unpark this thread on its own.
    notified: AtomicBool,
}

impl ThreadWaker {
    fn new_arc() -> Arc<Self> {
        Arc::new(Self {
            thread: thread::current(),
            notified: AtomicBool::new(false),
        })
    }

    fn park(&self) {
        // Block until a wakeup of ours is pending, consuming it, with Acquire to observe
        // everything the waker did before signalling us.
        //
        // Looping is required, not just tidy: `unpark` leaves a token in the thread's park slot
        // even when we were not parked yet, and a later `thread::park()` consumes that stale
        // token and returns straight away without us having been woken. Re-checking the flag
        // tells the two apart and parks again.
        while !self.notified.swap(false, Ordering::Acquire) {
            // self.thread.park() is private, but anyways we want to park the current thread.
            thread::park();
        }
    }

    fn unpark(&self) {
        // Publish with Release so the parked thread sees our work once it observes the flag.
        // Only hand out a token for the first wakeup of a park period: any further one would
        // still be pending and make the next `park` return early.
        if !self.notified.swap(true, Ordering::Release) {
            self.thread.unpark();
        }
    }
}

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.unpark()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.unpark()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{future, time::Duration};

    #[test]
    fn simple() {
        assert_eq!(simple_block_on(future::ready(42)), 42);
    }

    #[test]
    fn poll_fn() {
        let mut a = 0;
        let fut = future::poll_fn(move |cx| {
            if a == 5 {
                return Poll::Ready(10);
            }
            a += 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        });
        assert_eq!(simple_block_on(fut), 10);
    }

    // Both tests above wake from within the poll itself, which is the one case a waker that never
    // parks still handles. Waking from another thread is what actually requires parking, so drive
    // a future which only becomes ready that way and count the polls it takes.
    //
    // Two, exactly, either way: one which arms the waiter and parks, one which sees the flag. A
    // waker which spins instead of parking blows that up, and a spurious return from
    // `thread::park` does not, since the loop re-checks the flag and parks again.
    //
    // `plant_stale_token` stands in for driven code which parks and unparks this thread on its own
    // account: a bare `thread::park()` would take that leftover token for a wakeup of ours and
    // return without one, polling a third time while the future is still pending.
    //
    // Under a deadline because the other regression this guards against, a lost wakeup, blocks in
    // park forever rather than spinning, and would hang the test binary instead of failing it.
    fn poll_count_waking_from_another_thread(plant_stale_token: bool) -> u32 {
        let (out, polls) = crate::util::test::with_timeout(move || {
            let ready = Arc::new(AtomicBool::new(false));
            let mut polls = 0_u32;
            let mut waiter = None;

            let out = simple_block_on(future::poll_fn(|cx| {
                polls += 1;
                if ready.load(Ordering::Acquire) {
                    return Poll::Ready(42);
                }
                if waiter.is_none() {
                    if plant_stale_token {
                        thread::current().unpark();
                    }
                    let ready = ready.clone();
                    let waker = cx.waker().clone();
                    waiter = Some(thread::spawn(move || {
                        thread::sleep(Duration::from_millis(200));
                        ready.store(true, Ordering::Release);
                        waker.wake();
                    }));
                }
                Poll::Pending
            }));

            waiter.expect("waiter spawned").join().expect("waiter");
            (out, polls)
        });
        assert_eq!(out, 42);
        polls
    }

    #[test]
    fn parks_until_woken_from_another_thread() {
        let polls = poll_count_waking_from_another_thread(false);
        assert_eq!(polls, 2, "busy-spun instead of parking: {polls} polls");
    }

    #[test]
    fn ignores_a_park_token_it_did_not_hand_out() {
        let polls = poll_count_waking_from_another_thread(true);
        assert_eq!(polls, 2, "a stale park token was taken for a wakeup");
    }
}
