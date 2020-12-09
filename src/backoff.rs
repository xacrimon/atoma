use std::cell::Cell;
use std::sync::atomic;

const SPIN_LIMIT: u32 = 6;
const YIELD_LIMIT: u32 = 10;

pub struct Backoff {
    step: Cell<u32>
}

impl Backoff {
    pub fn new() -> Self {
        Self {
            step: Cell::new(0),
        }
    }

    pub fn reset(&self) {
        self.step.set(0);
    }

    pub fn spin(&self) {
        for _ in 0..1 << self.step.get().min(SPIN_LIMIT) {
            atomic::spin_loop_hint();
        }

        if self.step.get() <= SPIN_LIMIT {
            self.step.set(self.step.get() + 1);
        }
    }

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

    pub fn is_completed(&self) -> bool {
        self.step.get() > YIELD_LIMIT
    }
}
