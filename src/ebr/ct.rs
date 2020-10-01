use super::epoch::{AtomicEpoch, Epoch};
use super::global::Global;
use super::ADVANCE_PROBABILITY;
use crate::barrier::light_barrier;
use crate::CachePadded;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct CrossThread {
    lock: Mutex<()>,
    epoch: AtomicEpoch,
    shields: UnsafeCell<usize>,
    advance_counter: UnsafeCell<usize>,
}

impl CrossThread {
    pub(crate) fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            epoch: AtomicEpoch::new(Epoch::ZERO),
            shields: UnsafeCell::new(0),
            advance_counter: UnsafeCell::new(0),
        }
    }

    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.epoch.load(Ordering::Relaxed)
    }

    unsafe fn should_advance(&self, global: &Global) -> bool {
        let advance_counter = &mut *self.advance_counter.get();
        *advance_counter += 1;

        if *advance_counter == ADVANCE_PROBABILITY - 1 {
            *advance_counter = 0;
            global.should_advance()
        } else {
            false
        }
    }

    pub(crate) unsafe fn enter(&self, global: &Global) {
        let lock = self.lock.lock().unwrap();
        let shields = &mut *self.shields.get();
        let previous_shields = *shields;
        *shields += 1;

        if previous_shields == 0 {
            let global_epoch = global.load_epoch_relaxed();
            let new_epoch = global_epoch.pinned();
            self.epoch.store(new_epoch, Ordering::Relaxed);
        }

        drop(lock);
    }

    pub(crate) unsafe fn exit(&self, global: &Arc<Global>) {
        let lock = self.lock.lock().unwrap();
        let shields = &mut *self.shields.get();
        let previous_shields = *shields;
        *shields -= 1;

        if previous_shields == 1 {
            self.epoch.store(Epoch::ZERO, Ordering::Relaxed);
            self.finalize(global);
        }

        drop(lock);
    }

    unsafe fn finalize(&self, global: &Arc<Global>) {
        if self.should_advance(global) {
            let local_state = Global::local_state(global);
            let _ = global.try_cycle(local_state);
        }
    }
}
