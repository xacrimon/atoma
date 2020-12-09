use crate::{Backoff, CachePadded};
use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

const UNLOCKED: bool = false;
const LOCKED: bool = true;

pub struct Mutex<T> {
    state: CachePadded<AtomicBool>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            state: CachePadded::new(AtomicBool::new(UNLOCKED)),
            data: UnsafeCell::new(data),
        }
    }

    fn acquire(&self) {
        let backoff = Backoff::new();

        while self
            .state
            .compare_exchange_weak(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            backoff.snooze();
        }
    }

    fn release(&self) {
        self.state.store(UNLOCKED, Ordering::Release);
    }

    fn get_shared(&self) -> &T {
        unsafe { &*self.data.get() }
    }

    #[allow(clippy::mut_from_ref)]
    fn get_unique(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.acquire();
        MutexGuard::new(self)
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> MutexGuard<'a, T> {
    fn new(mutex: &'a Mutex<T>) -> Self {
        Self { mutex }
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.mutex.get_shared()
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.mutex.get_unique()
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.release();
    }
}

unsafe impl<'a, T: Send> Send for MutexGuard<'a, T> {}
unsafe impl<'a, T: Sync> Sync for MutexGuard<'a, T> {}
