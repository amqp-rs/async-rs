use std::{panic, sync::mpsc, thread, time::Duration};

// These take a reference so that callers can hand them a borrow without the bound landing on the
// reference instead of the type: `assert_send(&runtime)` against a by-value parameter asserts
// `&T: Send`, which is `T: Sync`, not what it reads as.

/// Assert that a type implements Send
pub(crate) fn assert_send<T: Send>(_t: &T) {}

/// Assert that a type implements Sync
pub(crate) fn assert_sync<T: Sync>(_t: &T) {}

/// Assert that a type implements Clone
pub(crate) fn assert_clone<T: Clone>(_t: &T) {}

/// Run `f` on a helper thread, failing the test if it does not finish in time.
///
/// Several of our regression tests guard against a future that never completes. Blocking on one
/// of those directly means the regression they are meant to catch hangs the test binary until CI
/// gives up, instead of reporting a failure, so give them a deadline the test itself enforces.
/// The helper thread is left behind on timeout; the process exits once the harness is done.
///
/// A panic in `f` is carried back to the calling thread rather than reported as a timeout: it
/// drops the sender without sending, which the receiver reports immediately, so blaming the
/// deadline would misdiagnose exactly the failures these tests exist to catch. Resuming it here
/// also puts the message back on the thread libtest is capturing output from.
pub(crate) fn with_timeout<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || tx.send(f()));
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(res) => res,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("timed out waiting for a future that should have completed")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => match handle.join() {
            Err(payload) => panic::resume_unwind(payload),
            Ok(_) => panic!("helper thread finished without handing back a result"),
        },
    }
}
