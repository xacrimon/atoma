use super::epoch::{AtomicEpoch, Epoch};
use crate::fastrng::FastRng;
use std::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    sync::atomic::{Ordering, self},
};

const IS_X86: bool = cfg!(any(target_arch = "x86", target_arch = "x86_64"));
const COLLECT_CHANCE: u32 = 128;

/// The interface we need in order to work with the main GC state.
pub trait EbrState {
    fn load_epoch_relaxed(&self) -> Epoch;
    fn should_advance(&self) -> bool;
    fn try_cycle(&self);
}

/// Per thread state needed for the GC.
/// We store a local epoch, an active flag and a number generator used
/// for reducing the frequency of some operations.
pub struct ThreadState<G> {
    /// A counter of how many shields are active.
    shields: UnsafeCell<Cell<u32>>,

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
        let global_epoch = state.load_epoch_relaxed();

        Self {
            shields: UnsafeCell::new(Cell::new(0)),
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

    /// Get the local epoch of the given thread.
    pub fn load_epoch_acquire(&self) -> Epoch {
        self.epoch.load(Ordering::Acquire)
    }

    /// Enter a critical section with the given thread.
    ///
    /// # Safety
    /// This function may only be called from the thread this state belongs to.
    pub unsafe fn enter(&self, state: &G) {
        let atomic_cell = &*self.shields.get();
        let previous_shields = atomic_cell.get();
        atomic_cell.set(previous_shields + 1);

        if previous_shields == 0 {
            let global_epoch = state.load_epoch_relaxed();
            let new_epoch = global_epoch.pinned();

            if IS_X86 {
                let current = Epoch::ZERO;
                let previous_epoch = self.epoch.compare_and_swap_seq_cst(current, new_epoch);
                debug_assert_eq!(current, previous_epoch);
                atomic::compiler_fence(Ordering::SeqCst);
            } else {
                self.epoch.store(new_epoch, Ordering::Relaxed);
                atomic::fence(Ordering::SeqCst);
            }
        }
    }

    /// Exit a critical section with the given thread.
    ///
    /// # Safety
    /// This function may only be called from the thread this state belongs to.
    pub unsafe fn exit(&self, state: &G) {
        let atomic_cell = &*self.shields.get();
        let previous_shields = atomic_cell.get();
        atomic_cell.set(previous_shields - 1);

        if previous_shields == 1 {
            self.epoch.store(Epoch::ZERO, Ordering::Release);

            if self.should_advance(state) {
                state.try_cycle();
            }
        }
    }
}

unsafe impl<G> Send for ThreadState<G> {}
unsafe impl<G> Sync for ThreadState<G> {}
