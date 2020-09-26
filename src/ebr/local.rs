use super::{
    epoch::{AtomicEpoch, Epoch},
    global::Global,
    shield::Shield,
};
use crate::{barrier::light_barrier, deferred::Deferred, CachePadded};
use std::{
    cell::UnsafeCell,
    sync::{atomic::Ordering, Arc},
};

const ADVANCE_PROBABILITY: u32 = 128;

pub(crate) struct LocalState {
    global: Arc<Global>,
    epoch: CachePadded<AtomicEpoch>,
    shields: UnsafeCell<u32>,
    advance_counter: UnsafeCell<u32>,
}

impl LocalState {
    pub(crate) fn new(global: Arc<Global>) -> Self {
        Self {
            global,
            epoch: CachePadded::new(AtomicEpoch::new(Epoch::ZERO)),
            shields: UnsafeCell::new(0),
            advance_counter: UnsafeCell::new(0),
        }
    }

    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.epoch.load(Ordering::Relaxed)
    }

    unsafe fn should_advance(&self) -> bool {
        let advance_counter = &mut *self.advance_counter.get();
        let previous_advance_counter = *advance_counter;

        if previous_advance_counter == ADVANCE_PROBABILITY - 1 {
            *advance_counter = 0;
            self.global.should_advance()
        } else {
            *advance_counter = previous_advance_counter + 1;
            false
        }
    }

    pub(crate) unsafe fn enter(&self) {
        let shields = &mut *self.shields.get();
        let previous_shields = *shields;
        *shields = previous_shields + 1;

        if previous_shields == 0 {
            let global_epoch = self.global.load_epoch_relaxed();
            let new_epoch = global_epoch.pinned();
            self.epoch.store(new_epoch, Ordering::Relaxed);
            light_barrier();
        }
    }

    pub(crate) unsafe fn exit(&self) {
        let shields = &mut *self.shields.get();
        let previous_shields = *shields;
        *shields = previous_shields - 1;

        if previous_shields == 1 {
            self.epoch.store(Epoch::ZERO, Ordering::Relaxed);
            self.finalize();
        }
    }

    unsafe fn finalize(&self) {
        let shields = &mut *self.shields.get();

        if self.should_advance() {
            *shields += 1;
            self.global.try_cycle(self);
            *shields -= 1;
        }
    }

    fn is_pinned(&self) -> bool {
        self.epoch.load(Ordering::Relaxed).is_pinned()
    }

    pub(crate) fn retire(&self, deferred: Deferred, shield: &Shield) {
        self.global.retire(deferred, shield);
    }

    pub(crate) fn shield(&self) -> Shield<'_> {
        unsafe {
            self.enter();
        }

        Shield::new(self)
    }
}

unsafe impl Send for LocalState {}
unsafe impl Sync for LocalState {}

pub struct Local {
    local_state: Arc<LocalState>,
}

impl Local {
    pub(crate) fn new(local_state: Arc<LocalState>) -> Self {
        Self { local_state }
    }

    pub fn shield(&self) -> Shield<'_> {
        self.local_state.shield()
    }

    pub fn is_pinned(&self) -> bool {
        self.local_state.is_pinned()
    }
}
