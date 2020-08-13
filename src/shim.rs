#[cfg(not(loom))]
pub use std::{sync, thread};

#[cfg(loom)]
pub use loom::{sync, thread};
