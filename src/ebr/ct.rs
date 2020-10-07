use super::epoch::{AtomicEpoch, Epoch};
use super::global::Global;
use super::ADVANCE_PROBABILITY;

use crate::mutex::Mutex;
use crate::CachePadded;
use std::cell::Cell;
use std::sync::atomic::Ordering;
use std::sync::Arc;

struct CtPaddedData {
    epoch: AtomicEpoch,
    shields: Cell<usize>,
    advance_counter: Cell<usize>,
}

pub struct CrossThread {
    lock: Mutex<()>,
    data: CachePadded<CtPaddedData>,
}

impl CrossThread {
    pub(crate) fn new() -> Self {
        let data = CachePadded::new(CtPaddedData {
            epoch: AtomicEpoch::new(Epoch::ZERO),
            shields: Cell::new(0),
            advance_counter: Cell::new(0),
        });

        Self {
            lock: Mutex::new(()),
            data,
        }
    }

    /// This loads the epoch. Since the epoch is stored atomically and
    /// we do not modify any other state we can avoid acquiring the lock here.
    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.data.epoch.load(Ordering::Relaxed)
    }

    /// # Safety
    ///
    /// The calling thread needs to hold the lock during this function call to
    /// synchronize access to variables.
    unsafe fn should_advance(&self, global: &Global) -> bool {
        let advance_counter_new = self.data.advance_counter.get() + 1;
        self.data.advance_counter.set(advance_counter_new);

        if advance_counter_new == ADVANCE_PROBABILITY - 1 {
            self.data.advance_counter.set(0);
            global.should_advance()
        } else {
            false
        }
    }

    /// This function records the creation of one full shield.
    ///
    /// It acquires the lock and can therefore modify variables and is guaranteed
    /// not to race with another thread executing this function.
    ///
    /// It is marked unsafe since calling it assumes exit will be called once
    /// at a later point in time.
    pub(crate) unsafe fn enter(&self, global: &Global) {
        let lock = self.lock.lock();
        let previous_shields = self.data.shields.get();
        self.data.shields.set(previous_shields + 1);

        if previous_shields == 0 {
            let global_epoch = global.load_epoch_relaxed();
            let new_epoch = global_epoch.pinned();
            self.data.epoch.store(new_epoch, Ordering::Relaxed);
        }

        drop(lock);
    }

    /// This function records the destruction of one full shield.
    ///
    /// Like above this modifies internal state and thus acquires the internal
    /// lock at teh start of the function.
    ///
    /// It is marked unsafe because it assumes enter has been called previously once for
    /// every corresponding call to this function.
    pub(crate) unsafe fn exit(&self, global: &Arc<Global>) {
        let lock = self.lock.lock();
        let previous_shields = self.data.shields.get();
        debug_assert!(previous_shields > 0);
        self.data.shields.set(previous_shields - 1);

        if previous_shields == 1 {
            self.data.epoch.store(Epoch::ZERO, Ordering::Relaxed);
            self.finalize(global);
        }

        drop(lock);
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
