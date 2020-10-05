use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{fence, AtomicBool, Ordering};

const LOCKED: bool = true;
const UNLOCKED: bool = false;

pub struct Mutex<T> {
    state: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            state: AtomicBool::new(UNLOCKED),
            value: UnsafeCell::new(value),
        }
    }

    unsafe fn acquire(&self) {
        while self.state.swap(LOCKED, Ordering::Relaxed) == LOCKED {}
        fence(Ordering::Acquire);
    }

    unsafe fn release(&self) {
        self.state.store(UNLOCKED, Ordering::Relaxed);
        fence(Ordering::Release);
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        unsafe {
            self.acquire();
        }

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

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.mutex.release();
        }
    }
}
