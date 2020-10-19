use super::epoch::{AtomicEpoch, Epoch};
use super::global::Global;
use crate::barrier::light_barrier;
use crate::CachePadded;
use std::sync::atomic::{fence, AtomicIsize, Ordering};
use std::sync::Arc;

pub struct CrossThread {
    epoch: CachePadded<AtomicEpoch>,
    shields: CachePadded<AtomicIsize>,
}

impl CrossThread {
    pub(crate) fn new() -> Self {
        Self {
            epoch: CachePadded::new(AtomicEpoch::new(Epoch::ZERO)),
            shields: CachePadded::new(AtomicIsize::new(0)),
        }
    }

    /// This loads the epoch. Since the epoch is stored atomically and
    /// we do not modify any other state we can avoid acquiring the lock here.
    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.epoch.load(Ordering::Relaxed)
    }

    /// # Safety
    ///
    /// The calling thread needs to hold the lock during this function call to
    /// synchronize access to variables.
    unsafe fn should_advance(&self, global: &Global) -> bool {
        global.should_advance()
    }

    /// This function records the creation of one full shield.
    ///
    /// It acquires the lock and can therefore modify variables and is guaranteed
    /// not to race with another thread executing this function.
    ///
    /// It is marked unsafe since calling it assumes exit will be called once
    /// at a later point in time.
    pub(crate) unsafe fn enter(&self, global: &Global) {
        let previous_shields = self.shields.fetch_add(1, Ordering::Relaxed);

        if previous_shields == 0 {
            let global_epoch = global.load_epoch_relaxed();
            let new_epoch = global_epoch.pinned();
            self.epoch.store(new_epoch, Ordering::Relaxed);
            light_barrier();
        }
    }

    /// This function records the destruction of one full shield.
    ///
    /// Like above this modifies internal state and thus acquires the internal
    /// lock at teh start of the function.
    ///
    /// It is marked unsafe because it assumes enter has been called previously once for
    /// every corresponding call to this function.
    pub(crate) unsafe fn exit(&self, global: &Arc<Global>) {
        let epoch_prev = self.epoch.load(Ordering::Relaxed);
        fence(Ordering::Acquire);
        let previous_shields = self.shields.fetch_sub(1, Ordering::Relaxed);

        if previous_shields == 0 {
            self.epoch
                .compare_and_set_non_unique(epoch_prev, Epoch::ZERO, Ordering::Relaxed);
            self.finalize(global);
        }
    }

    /// # Safety
    ///
    /// The calling thread needs to hold the lock during this function call to
    /// synchronize access to variables.
    unsafe fn finalize(&self, global: &Arc<Global>) {
        if self.should_advance(global) {
            let local_state = Global::local_state(global);
            let _ = global.try_cycle(local_state);
        }
    }
}
