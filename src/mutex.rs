use crate::CachePadded;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{fence, spin_loop_hint, AtomicUsize, Ordering};

pub struct Mutex<T> {
    next_ticket: CachePadded<AtomicUsize>,
    now_serving: CachePadded<AtomicUsize>,
    value: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            next_ticket: CachePadded::new(AtomicUsize::new(0)),
            now_serving: CachePadded::new(AtomicUsize::new(0)),
            value: UnsafeCell::new(value),
        }
    }

    unsafe fn acquire(&self) -> usize {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        while self.now_serving.load(Ordering::Relaxed) != ticket {
            spin_loop_hint();
        }

        fence(Ordering::Acquire);
        ticket
    }

    unsafe fn release(&self, ticket: usize) {
        let next_ticket = ticket.wrapping_add(1);
        self.now_serving.store(next_ticket, Ordering::Relaxed);
        fence(Ordering::Release);
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        let ticket = unsafe { self.acquire() };
        MutexGuard::new(self, ticket)
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    ticket: usize,
}

impl<'a, T> MutexGuard<'a, T> {
    fn new(mutex: &'a Mutex<T>, ticket: usize) -> Self {
        Self { mutex, ticket }
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
            self.mutex.release(self.ticket);
        }
    }
}
