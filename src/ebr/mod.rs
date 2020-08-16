mod epoch;
mod queue;
mod thread_state;

use crate::{
    shield::{CloneShield, Shield},
    thread_local::ThreadLocal,
    ReclaimableManager, Reclaimer,
};
use epoch::{AtomicEpoch, Epoch};
use queue::Queue;
use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{self, AtomicPtr, Ordering},
};
use thread_state::{EbrState, ThreadState};

type TypedQueue<T> = Queue<UnsafeCell<MaybeUninit<T>>>;

pub struct Ebr<M: ReclaimableManager> {
    epoch: AtomicEpoch,
    threads: ThreadLocal<ThreadState<Self>>,
    reclaimable_manager: M,
    queues: [AtomicPtr<TypedQueue<M::Reclaimable>>; 4],
}

impl<M: ReclaimableManager> Ebr<M> {
    pub fn new(reclaimable_manager: M) -> Self {
        Self {
            epoch: AtomicEpoch::new(Epoch::ZERO),
            threads: ThreadLocal::new(),
            reclaimable_manager,
            queues: [
                AtomicPtr::new(Queue::new()),
                AtomicPtr::new(Queue::new()),
                AtomicPtr::new(Queue::new()),
                AtomicPtr::new(Queue::new()),
            ],
        }
    }

    pub fn shield(&self) -> Shield<'_, Self> {
        unsafe {
            self.thread_state().enter(&self);
        }

        let state = ShieldState::new();
        Shield::new(&self, state)
    }

    fn get_queue(&self, epoch: Epoch) -> &TypedQueue<M::Reclaimable> {
        let raw_epoch = epoch.into_raw() as usize;
        let atomic_queue = unsafe { self.queues.get_unchecked(raw_epoch) };
        unsafe { &*atomic_queue.load(Ordering::Acquire) }
    }

    fn try_advance(&self) -> Result<Epoch, ()> {
        let global_epoch = self.epoch.load(Ordering::Relaxed);

        let can_collect = self
            .threads
            .iter()
            .map(|state| state.load_epoch_acquire())
            .filter(|epoch| epoch.is_pinned())
            .all(|epoch| epoch.unpinned() == global_epoch);

        if can_collect {
            self.epoch.try_advance(global_epoch)
        } else {
            Err(())
        }
    }

    unsafe fn collect(&self, epoch: Epoch, replace: bool) {
        let raw_epoch = epoch.into_raw() as usize;

        let new_queue_ptr = if replace {
            Queue::new()
        } else {
            ptr::null_mut()
        };

        let old_queue_ptr = self
            .queues
            .get_unchecked(raw_epoch)
            .swap(new_queue_ptr, Ordering::AcqRel);

        let mut maybe_queue = Some(&*old_queue_ptr);

        while let Some(queue) = maybe_queue {
            for cell in queue.iter() {
                let object = ptr::read(cell.get() as *mut M::Reclaimable);
                self.reclaimable_manager.reclaim(object)
            }

            maybe_queue = queue.get_next();
        }

        Box::from_raw(old_queue_ptr);
    }

    fn thread_state(&self) -> &ThreadState<Self> {
        self.threads.get(|| ThreadState::new(&self))
    }
}

pub struct ShieldState {
    _m0: PhantomData<*mut ()>,
}

impl ShieldState {
    fn new() -> Self {
        Self { _m0: PhantomData }
    }
}

impl<M: ReclaimableManager> CloneShield<Ebr<M>> for ShieldState {
    fn clone_shield(&self, reclaimer: &Ebr<M>) -> Self {
        unsafe {
            reclaimer.thread_state().enter(reclaimer);
        }

        ShieldState::new()
    }
}

unsafe impl Sync for ShieldState {}

impl<M: ReclaimableManager> Reclaimer for Ebr<M> {
    type ShieldState = ShieldState;
    type Reclaimable = M::Reclaimable;

    fn drop_shield(&self, _state: &mut Self::ShieldState) {
        unsafe {
            self.thread_state().exit(&self);
        }
    }

    fn retire(&self, _state: &Self::ShieldState, param: Self::Reclaimable) {
        atomic::fence(Ordering::SeqCst);
        let item = UnsafeCell::new(MaybeUninit::new(param));
        let epoch = self.epoch.load(Ordering::Relaxed);
        self.get_queue(epoch).push(item);
    }
}

impl<M: ReclaimableManager> EbrState for Ebr<M> {
    fn load_epoch_relaxed(&self) -> Epoch {
        self.epoch.load(Ordering::Relaxed)
    }

    fn should_advance(&self) -> bool {
        let epoch = self.epoch.load(Ordering::Acquire);
        let queue = self.get_queue(epoch);
        queue.len() >= (queue.capacity() / 2)
    }

    fn try_cycle(&self) {
        if let Ok(epoch) = self.try_advance() {
            let safe_epoch = epoch.next();

            unsafe {
                self.collect(safe_epoch, true);
            }
        }
    }
}

impl<M: ReclaimableManager> Drop for Ebr<M> {
    fn drop(&mut self) {
        unsafe {
            self.collect(Epoch::ZERO, false);
            self.collect(Epoch::ONE, false);
            self.collect(Epoch::TWO, false);
            self.collect(Epoch::THREE, false);
        }
    }
}
