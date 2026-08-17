// These take a reference so that callers can hand them a borrow without the bound landing on the
// reference instead of the type: `assert_send(&runtime)` against a by-value parameter asserts
// `&T: Send`, which is `T: Sync`, not what it reads as.

/// Assert that a type implements Send
pub(crate) fn assert_send<T: Send>(_t: &T) {}

/// Assert that a type implements Sync
pub(crate) fn assert_sync<T: Sync>(_t: &T) {}

/// Assert that a type implements Clone
pub(crate) fn assert_clone<T: Clone>(_t: &T) {}
