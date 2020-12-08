use super::{
    bag::Bag,
    epoch::{AtomicEpoch, Epoch},
    global::Global,
    shield::{Shield, ThinShield},
    ADVANCE_PROBABILITY,
};
use crate::{barrier::light_barrier, deferred::Deferred, CachePadded};
use core::{cell::UnsafeCell, fmt, marker::PhantomData, mem, sync::atomic::Ordering};
use std::sync::Arc;

pub(crate) struct LocalState {
    global: Arc<Global>,
    epoch: CachePadded<AtomicEpoch>,
    shields: UnsafeCell<usize>,
    advance_counter: UnsafeCell<usize>,
    bag: UnsafeCell<Bag>,
}

impl LocalState {
    pub(crate) fn new(global: Arc<Global>) -> Self {
        Self {
            global,
            epoch: CachePadded::new(AtomicEpoch::new(Epoch::ZERO)),
            shields: UnsafeCell::new(0),
            advance_counter: UnsafeCell::new(0),
            bag: UnsafeCell::new(Bag::new()),
        }
    }

    /// This function loads the epoch without any ordering constraints.
    /// This may be called from any thread as it does not access non synchronized data.
    pub(crate) fn load_epoch_relaxed(&self) -> Epoch {
        self.epoch.load(Ordering::Relaxed)
    }

    /// # Safety
    ///
    /// This modifies internal state.
    /// It may only be called from the thread owning this `LocalState` instance.
    unsafe fn should_advance(&self) -> bool {
        let advance_counter = &mut *self.advance_counter.get();
        *advance_counter += 1;

        if *advance_counter != ADVANCE_PROBABILITY {
            false
        } else {
            *advance_counter = 0;
            self.global.should_advance()
        }
    }

    /// Records the creation of one thin shield. A call to this
    /// function must cause a corresponding call to `LocalState::exit` at a later point in time.
    ///
    /// # Safety
    ///
    /// This modifies internal state.
    /// It may only be called from the thread owning this `LocalState` instance.
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

    /// Records the creation of one thin shield. A call to this
    /// function must correspond to a call to `LocalState::enter` that occured at
    /// an earlier point in time.
    ///
    /// # Safety
    ///
    /// This modifies internal state.
    /// It may only be called from the thread owning this `LocalState` instance.
    pub(crate) unsafe fn exit(&self) {
        let shields = &mut *self.shields.get();
        let previous_shields = *shields;
        *shields = previous_shields - 1;

        if previous_shields == 1 {
            self.epoch.store(Epoch::ZERO, Ordering::Relaxed);
            self.finalize();
        }
    }

    /// # Safety
    ///
    /// This modifies internal state.
    /// It may only be called from the thread owning this `LocalState` instance.
    unsafe fn finalize(&self) {
        let shields = &mut *self.shields.get();

        if self.should_advance() {
            *shields += 1;
            let _ = self.global.try_cycle(self);
            *shields -= 1;
        }
    }

    fn is_pinned(&self) -> bool {
        self.epoch.load(Ordering::Relaxed).is_pinned()
    }

    pub(crate) fn retire<'a, S>(&self, deferred: Deferred, shield: &S)
    where
        S: Shield<'a>,
    {
        let epoch = self.global.load_epoch_relaxed();
        let bag = unsafe { &mut *self.bag.get() };
        bag.try_process(epoch);
        bag.push(deferred, epoch);

        if bag.is_full() {
            self.force_flush(shield);
        }
    }

    pub(crate) fn flush<'a, S>(&self, shield: &S)
    where
        S: Shield<'a>,
    {
        let bag = unsafe { &mut *self.bag.get() };

        if !bag.is_full() {
            self.force_flush(shield);
        }
    }

    fn force_flush<'a, S>(&self, shield: &S)
    where
        S: Shield<'a>,
    {
        let bag = unsafe { &mut *self.bag.get() };
        let sealed = mem::replace(bag, Bag::new()).seal();
        self.global.retire_bag(sealed, shield);
    }

    pub(crate) fn thin_shield(&self) -> ThinShield<'_> {
        // we're creating a thin shield object so therefore we must record the creation of it
        unsafe {
            self.enter();
        }

        ThinShield::new(self)
    }
}

unsafe impl Send for LocalState {}
unsafe impl Sync for LocalState {}

/// A `Local` represents a participant in the epoch system with a local epoch and a counter of active shields.
/// If you are going to be creating a lot of shields and can keep around a `Local` it will be faster than calling
/// `Collector::shield` every time since it avoids a table lookup to find the correct `Local`.
pub struct Local {
    local_state: Arc<LocalState>,
    _m0: PhantomData<*mut ()>,
}

impl Local {
    pub(crate) fn new(local_state: Arc<LocalState>) -> Self {
        Self {
            local_state,
            _m0: PhantomData,
        }
    }

    /// Creates a shield on this local.
    pub fn thin_shield(&self) -> ThinShield<'_> {
        self.local_state.thin_shield()
    }

    /// Returns true if this local has active shields and it's epoch is pinned.
    pub fn is_pinned(&self) -> bool {
        self.local_state.is_pinned()
    }
}

impl fmt::Debug for Local {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Local { .. }")
    }
}
