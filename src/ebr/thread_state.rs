use super::epoch::{AtomicEpoch, Epoch};
use crate::fastrng::FastRng;
use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

const COLLECT_CHANCE: u32 = 4;

/// The interface we need in order to work with the main GC state.
pub trait EbrState {
    fn load_epoch(&self, order: Ordering) -> Epoch;
    fn should_advance(&self) -> bool;
    fn try_cycle(&self);
}

/// Per thread state needed for the GC.
/// We store a local epoch, an active flag and a number generator used
/// for reducing the frequency of some operations.
pub struct ThreadState<G> {
    /// A counter of how many shields are active.
    active: AtomicUsize,

    /// The local epoch of the thread.
    epoch: AtomicEpoch,

    /// A thread local RNG used for lowering the frequence of
    /// attempted global epoch advancements.
    rng: UnsafeCell<FastRng>,

    /// Marker 0.
    _m0: PhantomData<G>,
}

impl<G: EbrState> ThreadState<G> {
    pub fn new(state: &G, thread_id: u32) -> Self {
        let global_epoch = state.load_epoch(Ordering::Relaxed);

        Self {
            active: AtomicUsize::new(0),
            epoch: AtomicEpoch::new(global_epoch),
            rng: UnsafeCell::new(FastRng::new(thread_id)),
            _m0: PhantomData,
        }
    }

    /// Check if we should try to advance the global epoch.
    ///
    /// We use random numbers here to reduce the frequency of this returning true.
    /// We do this because advancing the epoch is a rather expensive operation.
    ///
    /// # Safety
    /// This function may only be called from the thread this state belongs to.
    /// This is due to the fact that it will access the thread-local
    /// PRNG without synchronization.
    unsafe fn should_advance(&self, state: &G) -> bool {
        let rng = &mut *self.rng.get();
        (rng.generate() % COLLECT_CHANCE == 0) && state.should_advance()
    }

    /// Check if the given thread is in a critical section.
    pub fn is_active(&self) -> bool {
        // acquire is used here so it is not reordered with calls to `enter` or `exit
        self.active.load(Ordering::Acquire) == 0
    }

    /// Get the local epoch of the given thread.
    pub fn load_epoch(&self, order: Ordering) -> Epoch {
        self.epoch.load(order)
    }

    /// Enter a critical section with the given thread.
    ///
    /// # Safety
    /// This function may only be called from the thread this state belongs to.
    pub unsafe fn enter(&self, state: &G) {
        // since `active` is a counter we only need to
        // update the local epoch when we go from 0 to something else
        //
        // seqcst is used here to perform the first half of the store-load fence
        if self.active.fetch_add(1, Ordering::SeqCst) == 0 {
            // seqcst is used here to perform the second half of the store-load fence
            let global_epoch = state.load_epoch(Ordering::SeqCst);

            // relaxed is okay here for two reasons
            // the first one is that a reordering between this store and an advancement
            // check will result in a fail anyway, hence this cannot cause an illegal advance
            // the second reason is that there is only one thread writing to this
            // atomic and there is a store-load fence preceeding this store
            // which will prevent reordering between two subsequent stores
            self.epoch.store(global_epoch, Ordering::Relaxed);
        }
    }

    /// Exit a critical section with the given thread.
    ///
    /// # Safety
    /// This function may only be called from the thread this state belongs to.
    pub unsafe fn exit(&self, state: &G) {
        // decrement the `active` counter and fetch the previous value
        //
        // we need release here to synchronize with calls to `load_epoch`
        let prev_active = self.active.fetch_sub(1, Ordering::Release);

        // if the counter wraps we've called exit more than enter which is not allowed
        debug_assert!(prev_active != 0);

        // check if we should try to advance the epoch if it reaches 0
        if prev_active == 1 {
            if self.should_advance(state) {
                state.try_cycle();
            }
        }
    }
}

unsafe impl<G> Send for ThreadState<G> {}
unsafe impl<G> Sync for ThreadState<G> {}
