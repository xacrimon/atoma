// LICENSE NOTICE: Most of this code has been copied from the crossbeam repository with the MIT license.

use core::{cell::Cell, sync::atomic};

const SPIN_LIMIT: u32 = 6;
const YIELD_LIMIT: u32 = 10;

/// A `Backoff` instance allows a thread to wait for an increasing amount of time
/// on failure in order to reduce contention in lock-free algorithms.
pub struct Backoff {
    step: Cell<u32>,
}

impl Backoff {
    /// Create a instance with an internal counter starting at 0.
    pub fn new() -> Self {
        Self { step: Cell::new(0) }
    }

    /// Reset the internal counter and thus the progression of wait times.
    pub fn reset(&self) {
        self.step.set(0);
    }

    /// Spin some time based on the internal counter and then increment it.
    pub fn spin(&self) {
        for _ in 0..1 << self.step.get().min(SPIN_LIMIT) {
            atomic::spin_loop_hint();
        }

        if self.step.get() <= SPIN_LIMIT {
            self.step.set(self.step.get() + 1);
        }
    }

    /// Wait some time based on the internal counter and then increment it.
    pub fn snooze(&self) {
        if self.step.get() <= SPIN_LIMIT {
            for _ in 0..1 << self.step.get() {
                atomic::spin_loop_hint();
            }
        } else {
            #[cfg(not(feature = "std"))]
            for _ in 0..1 << self.step.get() {
                atomic::spin_loop_hint();
            }

            #[cfg(feature = "std")]
            ::std::thread::yield_now();
        }

        if self.step.get() <= YIELD_LIMIT {
            self.step.set(self.step.get() + 1);
        }
    }

    /// If this method returns true, we've been waiting for long enough that another syncronization
    /// primitive should be used to wake this thread when it should continue.
    pub fn is_completed(&self) -> bool {
        self.step.get() > YIELD_LIMIT
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}
