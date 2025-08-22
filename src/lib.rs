#![deny(missing_docs, missing_debug_implementations, unsafe_code)]
#![allow(clippy::manual_async_fn)]

//! A Rust async runtime abstration library.
//!
//! ## Features
//!
//! - tokio: enable the tokio implementation *(default)*
//! - smol: enable the smol implementation
//! - async-global-executor: enable the async-global-executor implementation
//! - async-io: enable the async-io reactor implementation
//!
//! ## Example
//!
//! ```rust
//! use async_rs::{Runtime, TokioRuntime, traits::*};
//! use std::{io, time::Duration};
//!
//! async fn get_a(rt: &TokioRuntime) -> io::Result<u32> {
//!     rt.spawn_blocking(|| Ok(12)).await
//! }
//!
//! async fn get_b(rt: &TokioRuntime) -> io::Result<u32> {
//!     rt.spawn(async { Ok(30) }).await
//! }
//!
//! async fn tokio_main(rt: &TokioRuntime) -> io::Result<()> {
//!     let a = get_a(&rt).await?;
//!     let b = get_b(&rt).await?;
//!     rt.sleep(Duration::from_millis(500)).await;
//!     assert_eq!(a + b, 42);
//!     Ok(())
//! }
//!
//! fn main() -> io::Result<()> {
//!     let rt = Runtime::tokio()?;
//!     rt.block_on(tokio_main(&rt))
//! }
//! ```

mod runtime;
pub use runtime::*;

pub mod traits;

mod implementors;
#[cfg(any(
    feature = "async-global-executor",
    feature = "async-io",
    feature = "hickory-dns",
    feature = "smol",
    feature = "tokio"
))]
pub use implementors::*;

pub mod util;

mod sys;
