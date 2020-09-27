use super::{
    epoch::{AtomicEpoch, Epoch},
    local::{Local, LocalState},
    shield::{Shield, ThinShield},
};
use crate::{
    barrier::strong_barrier, deferred::Deferred, queue::Queue, thread_local::ThreadLocal,
    CachePadded,
};
use std::{
    mem::MaybeUninit,
    ptr,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc,
    },
};

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

pub(crate) struct Global {
    threads: ThreadLocal<Arc<LocalState>>,
    deferred: Queue<DeferredItem>,
    global_epoch: CachePadded<AtomicEpoch>,
    deferred_amount: CachePadded<AtomicIsize>,
}

impl Global {
    pub(crate) fn new() -> Self {
        Self {
            threads: ThreadLocal::new(),
            deferred: Queue::new(),
            global_epoch: CachePadded::new(AtomicEpoch::new(Epoch::ZERO)),
            deferred_amount: CachePadded::new(AtomicIsize::new(0)),
        }
    }

    fn local_state<'a>(this: &'a Arc<Self>) -> &'a Arc<LocalState> {
        this.threads
            .get(|| Arc::new(LocalState::new(Arc::clone(this))))
    }

    pub(crate) fn thin_shield<'a>(this: &'a Arc<Self>) -> ThinShield<'a> {
        let local_state = Self::local_state(this);
        local_state.thin_shield()
    }

    pub(crate) fn local(this: &Arc<Self>) -> Local {
        let local_state = Self::local_state(this);
        Local::new(Arc::clone(&local_state))
    }

    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.global_epoch.load(Ordering::Relaxed)
    }

    pub(crate) fn retire<'a, S>(&self, deferred: Deferred, shield: &S)
    where
        S: Shield<'a>,
    {
        let epoch = self.global_epoch.load(Ordering::Relaxed);
        let deferred = DeferredItem::new(epoch, deferred);
        self.deferred.push(deferred, shield);
        self.deferred_amount.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn should_advance(&self) -> bool {
        self.deferred_amount.load(Ordering::Relaxed) > 0
    }

    pub(crate) fn try_collect_light(this: &Arc<Self>) -> Result<usize, ()> {
        let local_state = Self::local_state(this);
        this.try_cycle(local_state)
    }

    pub(crate) fn try_cycle(&self, local_state: &LocalState) -> Result<usize, ()> {
        if let Ok(epoch) = self.try_advance() {
            let safe_epoch = epoch.next();
            let shield = local_state.thin_shield();

            unsafe { Ok(self.internal_collect(safe_epoch, &shield)) }
        } else {
            Err(())
        }
    }

    unsafe fn internal_collect(&self, epoch: Epoch, shield: &ThinShield) -> usize {
        let mut executed_amount = 0;

        while let Some(deferred) = self
            .deferred
            .pop_if(|deferred| deferred.epoch == epoch, shield)
        {
            deferred.as_ref_unchecked().execute();
            self.deferred_amount.fetch_sub(1, Ordering::Relaxed);
            executed_amount += 1;
        }

        executed_amount
    }

    fn try_advance(&self) -> Result<Epoch, ()> {
        let global_epoch = self.global_epoch.load(Ordering::Relaxed);
        strong_barrier();

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
}
