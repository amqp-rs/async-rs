/// Assert that a type implements Send
pub fn assert_send<T: Send>(_t: T) {}

/// Assert that a type implements Sync
pub fn assert_sync<T: Sync>(_t: T) {}

/// Assert that a type implements Clone
pub fn assert_clone<T: Clone>(_t: T) {}
