#![deny(missing_docs, missing_debug_implementations, unsafe_code)]
#![warn(unreachable_pub, unused_qualifications, unused_lifetimes)]
#![warn(
    clippy::must_use_candidate,
    clippy::unwrap_in_result,
    clippy::panic_in_result_fn
)]
#![allow(clippy::manual_async_fn)]

//! A Rust async runtime abstraction library.
//!
//! Provides a unified [`Runtime`] type and a set of [`traits`]
//! (`Executor`, `Reactor`, `Dns`, …) that abstract over Tokio, smol, and
//! async-global-executor. Applications select exactly one runtime via feature
//! flags; library crates depend on the trait objects and remain
//! runtime-agnostic.
//!
//! # Feature flags
//!
//! | Flag | Notes |
//! |------|-------|
//! | `tokio` *(default)* | Tokio runtime |
//! | `smol` | smol executor |
//! | `async-global-executor` | async-global-executor |
//! | `async-io` | async-io reactor (required by `smol`) |
//! | `hickory-dns` | Hickory DNS resolver (tokio only) |
//!
//! # Example
//!
//! ```rust
//! # #[cfg(feature="tokio")]
//! # {
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
//!     let a = get_a(rt).await?;
//!     let b = get_b(rt).await?;
//!     rt.sleep(Duration::from_millis(500)).await;
//!     assert_eq!(a + b, 42);
//!     Ok(())
//! }
//!
//! fn main() -> io::Result<()> {
//!     let rt = Runtime::tokio()?;
//!     rt.block_on(tokio_main(&rt))
//! }
//! # }
//! ```
//!
//! Note that the `io::Result` above is the *task's own* output: awaiting a
//! [`Task`](util::Task) leaves no room to report that the task itself failed,
//! so a task which panicked resumes its panic in the awaiting task, and
//! awaiting one which was canceled, or whose runtime went away, panics too.

mod runtime;
pub use runtime::*;

pub mod traits;

mod implementors;
pub use implementors::*;

pub mod util;

mod sys;
