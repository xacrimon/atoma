mod epoch;
mod queue;
mod shield;
mod thread_state;

use crate::{deferred::Deferred, thread_local::ThreadLocal};
use epoch::{AtomicEpoch, Epoch};
use queue::Queue;
pub use shield::Shield;
use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{self, AtomicPtr, Ordering},
};
use thread_state::{EbrState, ThreadState};

type TypedQueue<T> = Queue<UnsafeCell<MaybeUninit<T>>>;

fn new_queue<T>() -> *mut Queue<T> {
    Box::into_raw(Box::new(Queue::new()))
}

pub struct Collector {
    global_epoch: AtomicEpoch,
    threads: ThreadLocal<ThreadState<Self>>,
    deferred: [AtomicPtr<TypedQueue<Deferred>>; Epoch::AMOUNT],
}

impl Collector {
    pub fn new() -> Self {
        Self {
            global_epoch: AtomicEpoch::new(Epoch::ZERO),
            threads: ThreadLocal::new(),
            deferred: [
                AtomicPtr::new(new_queue()),
                AtomicPtr::new(new_queue()),
                AtomicPtr::new(new_queue()),
                AtomicPtr::new(new_queue()),
            ],
        }
    }

    pub fn shield(&self) -> Shield {
        Shield::new(self)
    }

    pub fn collect(&self) {
        self.try_cycle();
    }

    pub(crate) fn retire(&self, deferred: Deferred) {
        atomic::fence(Ordering::SeqCst);
        let item = UnsafeCell::new(MaybeUninit::new(deferred));
        let epoch = self.global_epoch.load(Ordering::Relaxed);
        self.get_queue(epoch).push(item);
    }

    fn get_queue(&self, epoch: Epoch) -> &TypedQueue<Deferred> {
        let raw_epoch = epoch.into_raw() as usize;
        let atomic_queue = unsafe { self.deferred.get_unchecked(raw_epoch) };
        unsafe { &*atomic_queue.load(Ordering::Acquire) }
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

    unsafe fn internal_collect(&self, epoch: Epoch, replace: bool) {
        let raw_epoch = epoch.into_raw() as usize;

        let new_queue_ptr = if replace {
            new_queue()
        } else {
            ptr::null_mut()
        };

        let old_queue_ptr = self
            .deferred
            .get_unchecked(raw_epoch)
            .swap(new_queue_ptr, Ordering::AcqRel);

        let maybe_queue = Some(&*old_queue_ptr);

        if let Some(queue) = maybe_queue {
            for cell in queue.iter() {
                let deferred = ptr::read(cell.get() as *mut Deferred);
                deferred.call();
            }
        }

        Box::from_raw(old_queue_ptr);
    }

    pub(crate) fn thread_state(&self) -> &ThreadState<Self> {
        self.threads.get(|| ThreadState::new(&self))
    }
}

impl EbrState for Collector {
    fn load_epoch_relaxed(&self) -> Epoch {
        self.global_epoch.load(Ordering::Relaxed)
    }

    fn should_advance(&self) -> bool {
        let epoch = self.global_epoch.load(Ordering::Acquire);
        let queue = self.get_queue(epoch);
        queue.len() != 0
    }

    fn try_cycle(&self) {
        if let Ok(epoch) = self.try_advance() {
            let safe_epoch = epoch.next();

            unsafe {
                self.internal_collect(safe_epoch, true);
            }
        }
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        unsafe {
            self.internal_collect(Epoch::ZERO, false);
            self.internal_collect(Epoch::ONE, false);
            self.internal_collect(Epoch::TWO, false);
            self.internal_collect(Epoch::THREE, false);
        }
    }
}
