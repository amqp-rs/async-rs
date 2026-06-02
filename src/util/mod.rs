//! A collection of utilities to deal with IO, futures and runtimes

mod addr;
pub use addr::*;

mod block_on;
pub use block_on::*;

mod dummy;
pub use dummy::*;

mod join;
pub use join::*;

mod split;
pub use split::*;

#[cfg(feature = "async-io")]
mod io;
#[cfg(feature = "async-io")]
pub use io::*;

mod runtime;
pub use runtime::*;

mod task;
pub use task::*;

#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use tokio::*;

#[cfg(test)]
pub(crate) mod test;
