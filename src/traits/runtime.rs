use crate::Executor;

/// Supertrait to tag a type that implements all required components for a Runtime
pub trait RuntimeKit: Executor {} // TODO: require Reactor
