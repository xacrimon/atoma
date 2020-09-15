use super::epoch::{AtomicEpoch, Epoch};
use std::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    sync::atomic::{self, Ordering},
};

const IS_X86: bool = cfg!(any(target_arch = "x86", target_arch = "x86_64"));
const ADVANCE_PROBABILITY: usize = 128;

/// The interface we need in order to work with the main GC state.
pub trait EbrState {
    fn load_epoch_relaxed(&self) -> Epoch;
    fn should_advance(&self) -> bool;
    fn try_cycle(&self);
    fn collect_priority(&self) -> bool;
}

/// Per thread state needed for the GC.
/// We store a local epoch, an active flag and a number generator used
/// for reducing the frequency of some operations.
pub struct ThreadState<G> {
    /// A counter of how many shields are active.
    shields: UnsafeCell<Cell<u32>>,

    /// The local epoch of the thread.
    epoch: AtomicEpoch,

    /// A counter for periodically attempting to advance the epoch.
    advance_counter: UnsafeCell<Cell<usize>>,

    /// Marker 0.
    _m0: PhantomData<G>,
}

impl<G: EbrState> ThreadState<G> {
    pub fn new() -> Self {
        Self {
            shields: UnsafeCell::new(Cell::new(0)),
            epoch: AtomicEpoch::new(Epoch::ZERO),
            advance_counter: UnsafeCell::new(Cell::new(0)),
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
    /// counter without synchronization.
    unsafe fn should_advance(&self, state: &G) -> bool {
        let advance_counter_cell = &*self.advance_counter.get();
        let previous_advance_counter = advance_counter_cell.get();

        if previous_advance_counter == ADVANCE_PROBABILITY - 1 {
            advance_counter_cell.set(0);
            state.should_advance()
        } else {
            advance_counter_cell.set(previous_advance_counter + 1);
            false
        }
    }

    /// Get the local epoch of the given thread.
    pub fn load_epoch_relaxed(&self) -> Epoch {
        self.epoch.load(Ordering::Relaxed)
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
                let previous_epoch = self.epoch.swap_seq_cst(new_epoch);
                debug_assert_eq!(Epoch::ZERO, previous_epoch);
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
            let collect_priority = state.collect_priority();
            self.epoch.store(Epoch::ZERO, Ordering::Relaxed);

            if self.should_advance(state) || collect_priority {
                state.try_cycle();
            }
        }
    }
}

unsafe impl<G> Send for ThreadState<G> {}
unsafe impl<G> Sync for ThreadState<G> {}
