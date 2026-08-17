use std::{future::Future, io};
use tokio::runtime::Handle;

/// Check whether we're in a tokio context or not
#[must_use]
pub fn inside_tokio() -> bool {
    Handle::try_current().is_ok()
}

/// Block on the given future in a tokio context, creating a new one if required
///
/// A runtime created here lives only as long as the call. Anything the future leaves registered on
/// it — a socket, a timer, a resolver cached in a `static` — outlives its driver and is unusable by
/// the time the next call comes around, so only pass futures which keep nothing behind.
///
/// # Panics
///
/// Panics when called from an async context, as tokio refuses to block a thread it is driving
/// tasks on. Blocking threads, such as the one `spawn_blocking` runs its closure on, are fine.
pub fn block_on_tokio<T>(fut: impl Future<Output = io::Result<T>>) -> io::Result<T> {
    if let Ok(handle) = Handle::try_current() {
        handle.block_on(fut)
    } else {
        tokio::runtime::Runtime::new()?.block_on(fut)
    }
}
