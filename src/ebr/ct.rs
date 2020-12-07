use super::bag::{Bag, SealedBag};
use super::epoch::{AtomicEpoch, Epoch};
use super::global::Global;
use crate::barrier::light_barrier;
use crate::deferred::Deferred;
use crate::mutex::Mutex;
use crate::CachePadded;
use core::mem;
use core::sync::atomic::{fence, AtomicIsize, Ordering};
use std::sync::Arc;

pub struct CrossThread {
    epoch: CachePadded<AtomicEpoch>,
    shields: CachePadded<AtomicIsize>,
    bag: Mutex<Bag>,
}

impl CrossThread {
    pub(crate) fn new() -> Self {
        Self {
            epoch: CachePadded::new(AtomicEpoch::new(Epoch::ZERO)),
            shields: CachePadded::new(AtomicIsize::new(0)),
            bag: Mutex::new(Bag::new()),
        }
    }

    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.epoch.load(Ordering::Relaxed)
    }

    unsafe fn should_advance(&self, global: &Global) -> bool {
        global.should_advance()
    }

    pub(crate) unsafe fn enter(&self, global: &Global) {
        let previous_shields = self.shields.fetch_add(1, Ordering::Relaxed);

        if previous_shields == 0 {
            let global_epoch = global.load_epoch_relaxed();
            let new_epoch = global_epoch.pinned();
            self.epoch.store(new_epoch, Ordering::Relaxed);
            light_barrier();
        }
    }

    pub(crate) unsafe fn exit(&self, global: &Arc<Global>) {
        let epoch_prev = self.epoch.load(Ordering::Relaxed);
        fence(Ordering::Acquire);
        let previous_shields = self.shields.fetch_sub(1, Ordering::Relaxed);

        if previous_shields == 1 {
            self.epoch
                .compare_and_set_non_unique(epoch_prev, Epoch::ZERO, Ordering::Relaxed);

            self.finalize(global);
        }
    }

    unsafe fn finalize(&self, global: &Arc<Global>) {
        if self.should_advance(global) {
            let local_state = Global::local_state(global);
            let _ = global.try_cycle(local_state);
        }
    }

    pub(crate) fn retire(&self, deferred: Deferred, epoch: Epoch) -> Option<SealedBag> {
        let mut bag = self.bag.lock();
        bag.try_process(epoch);
        bag.push(deferred, epoch);

        if bag.is_full() {
            Some(Self::i_flush(&mut bag, epoch))
        } else {
            None
        }
    }

    pub(crate) fn flush(&self, epoch: Epoch) -> Option<SealedBag> {
        let mut bag = self.bag.lock();

        if !bag.is_empty() {
            Some(Self::i_flush(&mut bag, epoch))
        } else {
            None
        }
    }

    fn i_flush(bag: &mut Bag, epoch: Epoch) -> SealedBag {
        mem::replace(bag, Bag::new()).seal(epoch)
    }
}
