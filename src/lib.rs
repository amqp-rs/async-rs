#![forbid(unsafe_code)]
#![deny(missing_docs, missing_debug_implementations)]
#![no_std]

//! A collection of traits and implementations to define a common interface across async runtimes

extern crate alloc;
extern crate core;

pub mod executor;

#[cfg(feature = "tokio")]
pub mod tokio;
