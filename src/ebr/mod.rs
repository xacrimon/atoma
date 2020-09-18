mod epoch;
mod shield;
mod thread_state;

use crate::queue::Queue;
use crate::{deferred::Deferred, thread_local::ThreadLocal};
use epoch::{AtomicEpoch, Epoch};
pub use shield::{CowShield, Shield};
use std::{
    cmp,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{fence, AtomicIsize, AtomicUsize, Ordering},
};
use thread_state::{EbrState, ThreadState};

struct DeferredItem {
    epoch: Epoch,
    deferred: MaybeUninit<Deferred>,
}

impl DeferredItem {
    fn new(epoch: Epoch, deferred: Deferred) -> Self {
        Self {
            epoch,
            deferred: MaybeUninit::new(deferred),
        }
    }

    unsafe fn execute(&self) {
        ptr::read(&self.deferred).assume_init().call();
    }
}

unsafe impl Send for DeferredItem {}
unsafe impl Sync for DeferredItem {}

pub struct Collector {
    global_epoch: AtomicEpoch,
    threads: ThreadLocal<ThreadState<Self>>,
    deferred: Queue<DeferredItem>,
    deferred_amount: AtomicIsize,
    collect_amount_heuristic: AtomicUsize,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            global_epoch: AtomicEpoch::new(Epoch::ZERO),
            threads: ThreadLocal::new(),
            deferred: Queue::new(),
            deferred_amount: AtomicIsize::new(0),
            collect_amount_heuristic: AtomicUsize::new(0),
        }
    }

    pub fn shield(&self) -> Shield {
        Shield::new(self)
    }

    pub fn collect(&self) {
        let shield = self.shield();

        if self.should_advance() {
            self.try_cycle(&shield);
        }
    }

    pub(crate) fn retire(&self, deferred: Deferred, shield: &Shield) {
        let epoch = self.global_epoch.load(Ordering::Relaxed);
        let deferred = DeferredItem::new(epoch, deferred);
        self.deferred.push(deferred, shield);
        self.deferred_amount.fetch_add(1, Ordering::Relaxed);

        if self.priority_collect() {
            self.try_cycle(shield);
        }
    }

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

    unsafe fn internal_collect(&self, epoch: Epoch, shield: &Shield) {
        fence(Ordering::SeqCst);
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

    pub(crate) fn thread_state(&self) -> &ThreadState<Self> {
        self.threads.get(ThreadState::new)
    }

    fn get_collect_threshold(&self) -> usize {
        let last_collected_amount = self.collect_amount_heuristic.load(Ordering::Relaxed);
        let scaled_threshold = last_collected_amount / 2;
        cmp::max(scaled_threshold, 4)
    }

    fn priority_collect(&self) -> bool {
        let deferred_amount = self.deferred_amount.load(Ordering::Relaxed);
        let last_collected_amount = self.collect_amount_heuristic.load(Ordering::Relaxed);
        let priority_threshold = last_collected_amount * 2;
        deferred_amount > priority_threshold as isize
    }
}

impl EbrState for Collector {
    fn load_epoch_relaxed(&self) -> Epoch {
        self.global_epoch.load(Ordering::Relaxed)
    }

    fn should_advance(&self) -> bool {
        let deferred_amount = self.deferred_amount.load(Ordering::Relaxed);
        let collect_threshold = self.get_collect_threshold() as isize;
        deferred_amount > collect_threshold
    }

    fn try_cycle(&self, shield: &Shield) {
        if let Ok(epoch) = self.try_advance() {
            let safe_epoch = epoch.next();

            unsafe {
                self.internal_collect(safe_epoch, shield);
            }
        }
    }

    fn shield(&self) -> Shield {
        self.shield()
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        let shield = self.shield();
        while self.deferred.pop_if(|_| true, &shield).is_some() {}
    }
}

unsafe impl Send for Collector {}
unsafe impl Sync for Collector {}
