use std::{future::Future, io};
use tokio::runtime::Handle;

/// Check whether we're in a tokio context or not
pub fn inside_tokio() -> bool {
    Handle::try_current().is_ok()
}

/// Block on the given future in a tokio context, creating a new one if required
pub fn block_on_tokio<T>(fut: impl Future<Output = io::Result<T>>) -> io::Result<T> {
    if let Ok(handle) = Handle::try_current() {
        handle.block_on(fut)
    } else {
        tokio::runtime::Runtime::new()?.block_on(fut)
    }
}
