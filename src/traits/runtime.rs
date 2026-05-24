use crate::traits::{Executor, Reactor};

/// Marker supertrait for types that satisfy all requirements of a full async runtime.
///
/// A type implements `RuntimeKit` when it is both an [`Executor`] (can spawn and
/// block on futures) and a [`Reactor`] (can perform async I/O and timers) and is
/// [`Debug`](std::fmt::Debug). This trait has no methods of its own; it exists
/// purely as a convenient single bound used by [`Runtime`](crate::Runtime) and
/// [`RuntimeParts`](crate::util::RuntimeParts).
pub trait RuntimeKit: Executor + Reactor + std::fmt::Debug {}
