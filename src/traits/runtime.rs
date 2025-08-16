use crate::traits::{Executor, Reactor};

/// Supertrait to tag a type that implements all required components for a Runtime
pub trait RuntimeKit: Executor + Reactor + std::fmt::Debug {}
