//! A collection of utilities to deal with IO, futures and runtimes

mod future;
pub use future::*;

#[cfg(feature = "async-io")]
mod io;
#[cfg(feature = "async-io")]
pub use io::*;

mod runtime;
pub use runtime::*;
