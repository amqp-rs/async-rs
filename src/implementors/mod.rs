#[cfg(feature = "async-global-executor")]
mod async_global_executor;
#[cfg(feature = "async-global-executor")]
pub use async_global_executor::*;

#[cfg(feature = "async-io")]
mod async_io;
#[cfg(feature = "async-io")]
pub use async_io::*;

#[cfg(feature = "hickory-dns")]
/// Utilities to perform async DNS resolution using hickory DNS
pub mod hickory;

#[cfg(feature = "smol")]
mod smol;
#[cfg(feature = "smol")]
pub use smol::*;

#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use tokio::*;
