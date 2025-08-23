//! A collection of utilities to deal with IO, futures and runtimes

mod addr;
pub use addr::*;

mod future;
pub use future::*;

#[cfg(feature = "async-io")]
mod io;
#[cfg(feature = "async-io")]
pub use io::*;

mod runtime;
pub use runtime::*;

#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use tokio::*;

#[cfg(test)]
pub(crate) mod test;
