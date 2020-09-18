mod epoch;
mod shield;
mod thread_state;

use crate::{
    barrier::strong_barrier, deferred::Deferred, queue::Queue, thread_local::ThreadLocal,
    CachePadded,
};
use epoch::{AtomicEpoch, Epoch};
pub use shield::{CowShield, Shield};
use std::{
    cmp,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{AtomicIsize, AtomicUsize, Ordering},
};
use thread_state::{EbrState, ThreadState};

struct DeferredItem {
    epoch: Epoch,
    deferred: MaybeUninit<Deferred>,
}

impl DeferredItem {
    #[inline]
    fn new(epoch: Epoch, deferred: Deferred) -> Self {
        Self {
            epoch,
            deferred: MaybeUninit::new(deferred),
        }
    }

    #[inline]
    unsafe fn execute(&self) {
        ptr::read(&self.deferred).assume_init().call();
    }
}

unsafe impl Send for DeferredItem {}
unsafe impl Sync for DeferredItem {}

pub struct Collector {
    threads: ThreadLocal<ThreadState<Self>>,
    deferred: Queue<DeferredItem>,
    global_epoch: CachePadded<AtomicEpoch>,
    deferred_amount: CachePadded<AtomicIsize>,
    collect_amount_heuristic: CachePadded<AtomicUsize>,
}

impl Collector {
    #[cold]
    #[inline(never)]
    pub fn new() -> Self {
        Self {
            threads: ThreadLocal::new(),
            deferred: Queue::new(),
            global_epoch: CachePadded::new(AtomicEpoch::new(Epoch::ZERO)),
            deferred_amount: CachePadded::new(AtomicIsize::new(0)),
            collect_amount_heuristic: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    #[inline]
    pub fn shield(&self) -> Shield {
        Shield::new(self)
    }

    #[cold]
    #[inline(never)]
    pub fn try_collect_light(&self) {
        let shield = self.shield();
        self.try_cycle(&shield);
    }

    #[cold]
    #[inline(never)]
    pub fn try_collect_all(&self) {
        let shield = self.shield();
        let mut failures = 0;
        let mut left = Epoch::AMOUNT;

        while left != 0 {
            if self.try_cycle(&shield) {
                left -= 1;
            }

            if failures > 10 {
                break;
            }

            failures += 1;
        }
    }

    #[inline]
    pub(crate) fn retire(&self, deferred: Deferred, shield: &Shield) {
        let epoch = self.global_epoch.load(Ordering::Relaxed);
        let deferred = DeferredItem::new(epoch, deferred);
        self.deferred.push(deferred, shield);
        self.deferred_amount.fetch_add(1, Ordering::Relaxed);

        if self.priority_collect() {
            self.try_cycle(shield);
        }
    }

    #[cold]
    #[inline(never)]
    fn try_advance(&self) -> Result<Epoch, ()> {
        let global_epoch = self.global_epoch.load(Ordering::Relaxed);

        let can_collect = self
            .threads
            .iter()
            .map(|state| state.load_epoch_relaxed())
            .filter(|epoch| epoch.is_pinned())
            .all(|epoch| epoch.unpinned() == global_epoch);

        if can_collect {
            self.global_epoch.try_advance(global_epoch)
        } else {
            Err(())
        }
    }

    #[cold]
    #[inline(never)]
    unsafe fn internal_collect(&self, epoch: Epoch, shield: &Shield) {
        strong_barrier();
        let collect_amount_heuristic = self.collect_amount_heuristic.load(Ordering::Relaxed);
        let mut collected_amount = 0;

        while let Some(deferred) = self
            .deferred
            .pop_if(|deferred| deferred.epoch == epoch, shield)
        {
            deferred.as_ref_unchecked().execute();
            self.deferred_amount.fetch_sub(1, Ordering::Relaxed);
            collected_amount += 1;
        }

        if collected_amount > 2 {
            self.collect_amount_heuristic.compare_and_swap(
                collect_amount_heuristic,
                collected_amount,
                Ordering::Relaxed,
            );
        }
    }

    #[inline]
    pub(crate) fn thread_state(&self) -> &ThreadState<Self> {
        self.threads.get(ThreadState::new)
    }

    #[inline]
    fn get_collect_threshold(&self) -> usize {
        let last_collected_amount = self.collect_amount_heuristic.load(Ordering::Relaxed);
        let scaled_threshold = last_collected_amount / 2;
        cmp::max(scaled_threshold, 4)
    }

    #[inline]
    fn priority_collect(&self) -> bool {
        let deferred_amount = self.deferred_amount.load(Ordering::Relaxed);
        let last_collected_amount = self.collect_amount_heuristic.load(Ordering::Relaxed);
        let priority_threshold = last_collected_amount * 2;
        deferred_amount > cmp::max(priority_threshold, 16) as isize
    }
}

impl EbrState for Collector {
    #[inline]
    fn load_epoch_relaxed(&self) -> Epoch {
        self.global_epoch.load(Ordering::Relaxed)
    }

    #[cold]
    #[inline(never)]
    fn should_advance(&self) -> bool {
        let deferred_amount = self.deferred_amount.load(Ordering::Relaxed);
        let collect_threshold = self.get_collect_threshold() as isize;
        deferred_amount > collect_threshold
    }

    #[cold]
    #[inline(never)]
    fn try_cycle(&self, shield: &Shield) -> bool {
        if let Ok(epoch) = self.try_advance() {
            let safe_epoch = epoch.next();

            unsafe {
                self.internal_collect(safe_epoch, shield);
            }

            true
        } else {
            false
        }
    }

    #[inline]
    fn shield(&self) -> Shield {
        self.shield()
    }
}

impl Default for Collector {
    #[cold]
    #[inline(never)]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Collector {
    #[cold]
    #[inline(never)]
    fn drop(&mut self) {
        let shield = self.shield();
        while self.deferred.pop_if(|_| true, &shield).is_some() {}
    }
}

unsafe impl Send for Collector {}
unsafe impl Sync for Collector {}
