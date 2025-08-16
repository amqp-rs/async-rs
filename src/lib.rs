#![forbid(unsafe_code)]
#![deny(missing_docs, missing_debug_implementations)]
#![no_std]

//! A collection of traits and implementations to define a common interface across async runtimes

extern crate alloc;
extern crate core;

mod runtime;
pub use runtime::*;

mod traits;
pub use traits::*;

mod implementors;
pub use implementors::*;
