use super::{
    bag::SealedBag,
    ct::CrossThread,
    epoch::{AtomicEpoch, Epoch},
    local::{Local, LocalState},
    shield::{FullShield, Shield, ThinShield},
};
use crate::heap::Arc;
use crate::{
    alloc::AllocRef, barrier::strong_barrier, deferred::Deferred, queue::Queue, tls2::ThreadLocal,
    tls2::TlsProvider, CachePadded,
};
use core::{
    mem,
    sync::atomic::{fence, AtomicIsize, Ordering},
};

fn deferred_ceiling(max_bytes: usize) -> usize {
    max_bytes / mem::size_of::<Deferred>()
}

pub(crate) struct Global {
    threads: ThreadLocal<Arc<LocalState>>,
    deferred: Queue<SealedBag>,
    global_epoch: CachePadded<AtomicEpoch>,
    deferred_amount: CachePadded<AtomicIsize>,
    deferred_amount_ceiling: usize,
    pub(crate) ct: CrossThread,
    pub(crate) allocator: AllocRef,
}

impl Global {
    pub(crate) fn new(
        allocator: AllocRef,
        tls_provider: &'static dyn TlsProvider,
        max_garbage_bytes: usize,
    ) -> Self {
        Self {
            threads: ThreadLocal::new(tls_provider, allocator.clone()),
            deferred: Queue::new(allocator.clone()),
            global_epoch: CachePadded::new(AtomicEpoch::new(Epoch::ZERO)),
            deferred_amount: CachePadded::new(AtomicIsize::new(0)),
            deferred_amount_ceiling: deferred_ceiling(max_garbage_bytes),
            ct: CrossThread::new(),
            allocator,
        }
    }

    pub(crate) fn local_state(this: &Arc<Self>) -> &Arc<LocalState> {
        this.threads
            .get(|| Arc::new(LocalState::new(Arc::clone(this)), this.allocator.clone()))
    }

    pub(crate) fn thin_shield(this: &Arc<Self>) -> ThinShield<'_> {
        let local_state = Self::local_state(this);
        local_state.thin_shield()
    }

    pub(crate) fn full_shield(this: &Arc<Self>) -> FullShield<'_> {
        unsafe {
            this.ct.enter(this);
        }

        FullShield::new(this)
    }

    pub(crate) fn local(this: &Arc<Self>) -> Local {
        let local_state = Self::local_state(this);
        Local::new(Arc::clone(&local_state))
    }

    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.global_epoch.load(Ordering::Relaxed)
    }

    pub(crate) fn retire_bag<'a, S>(&self, bag: SealedBag, _shield: &S)
    where
        S: Shield<'a>,
    {
        let _epoch = self.global_epoch.load(Ordering::Relaxed);
        let diff = bag.len() as isize;
        self.deferred.push(bag);
        let len = self.deferred_amount.fetch_add(diff, Ordering::Relaxed);

        if len as usize > self.deferred_amount_ceiling {
            let _ = self.try_cycle();
        }
    }

    pub(crate) fn should_advance(&self) -> bool {
        self.deferred_amount.load(Ordering::Relaxed) > 0
    }

    pub(crate) fn try_collect_light(this: &Arc<Self>) -> bool {
        let local_state = Self::local_state(this);
        let shield = local_state.shield();
        let cycled = this.try_cycle();
        drop(shield);
        cycled
    }

    // Some sort of shield must be held for the duration of this call.
    pub(crate) fn try_cycle(&self) -> bool {
        if let Ok(epoch) = self.try_advance() {
            fence(Ordering::SeqCst);
            let cleaned = unsafe { self.internal_collect(epoch) };
            self.deferred_amount
                .fetch_sub(cleaned as isize, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    unsafe fn internal_collect(&self, epoch: Epoch) -> usize {
        let mut executed_amount = 0;

        while let Some(sealed) = self.deferred.pop() {
            if sealed.epoch().has_passed(epoch, 2) {
                executed_amount += sealed.run();
            } else {
                self.deferred.push(sealed);
                break;
            }
        }

        executed_amount
    }

    fn try_advance(&self) -> Result<Epoch, ()> {
        let global_epoch = self.global_epoch.load(Ordering::Relaxed);
        let snapshot = self.threads.snapshot();
        strong_barrier();
        let ct_epoch = self.ct.load_epoch_relaxed();
        let ct_is_sync = !ct_epoch.is_pinned() || ct_epoch == global_epoch;

        let synced_epochs = self
            .threads
            .iter()
            .map(|state| state.load_epoch_relaxed())
            .filter(|epoch| epoch.is_pinned())
            .all(|epoch| epoch.unpinned() == global_epoch);

        if synced_epochs && ct_is_sync && !self.threads.changed_since(snapshot) {
            self.global_epoch.try_advance(global_epoch)
        } else {
            Err(())
        }
    }
}
