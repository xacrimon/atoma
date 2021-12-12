use core::{cell::Cell, hint};

const SPIN_LIMIT: u32 = 6;

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

    /// Wait some period of time based on the internal counter and then increment it.
    pub fn snooze(&self) {
        if self.step.get() <= SPIN_LIMIT {
            for _ in 0..(1 << self.step.get()) {
                hint::spin_loop();
            }

            self.step.set(self.step.get() + 1);
        } else {
            std::thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Backoff;

    #[test]
    fn backoff() {
        let backoff = Backoff::new();

        for _ in 0..100 {
            backoff.snooze();
        }
    }
}
