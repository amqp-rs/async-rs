use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake},
    thread::{self, Thread},
};

/// Simple naive block_on implementation for noop runtime
pub fn simple_block_on<F: Future>(f: F) -> F::Output {
    let _enter = enter();
    let thread = ThreadWaker::new_arc();
    let waker = thread.clone().into();
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(r) => return r,
            Poll::Pending => thread.park(),
        }
    }
}

thread_local! {
    static BUSY: AtomicBool = const { AtomicBool::new(false) };
}

struct EnterGuard;

impl Drop for EnterGuard {
    fn drop(&mut self) {
        BUSY.with(|e| e.swap(false, Ordering::Acquire));
    }
}

fn enter() -> EnterGuard {
    if BUSY.with(|e| e.swap(true, Ordering::Release)) {
        panic!("Cannot call simple_block_on recursively")
    }

    EnterGuard
}

struct ThreadWaker {
    thread: Thread,
    parked: AtomicBool,
}

impl ThreadWaker {
    fn new_arc() -> Arc<Self> {
        Arc::new(Self {
            thread: thread::current(),
            parked: AtomicBool::new(true),
        })
    }

    fn park(&self) {
        // Check with Ordering Release to make sure we're ran first.
        // Better unpark once too much than park once too much.
        // Only park if we weren't already.
        if !self.parked.swap(true, Ordering::Acquire) {
            // self.thread.park() is private, but anyways we want to park the current thread.
            thread::park();
        }
    }

    fn unpark(&self) {
        // Check with Ordering Release to make sure we're ran last.
        // Better unpark once too much than park once too much.
        // Only unpark if we were parked.
        if self.parked.swap(false, Ordering::Release) {
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
    use std::future;

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
}
