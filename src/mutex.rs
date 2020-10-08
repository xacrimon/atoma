use crate::CachePadded;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{spin_loop_hint, AtomicUsize, Ordering};

/// This is an implementation of a fair mutex based on a ticket lock.
/// The choice of a ticket lock was made because we wanted to keep the worst case latency down.
///
/// This struct internally pads contended atomic variables to the cache line size.
///
/// The downside to this lock is that the average latency tanks when the amount of
/// waiting threads exceed the amount of processors on the system.
pub struct Mutex<T> {
    next_ticket: CachePadded<AtomicUsize>,
    now_serving: CachePadded<AtomicUsize>,
    value: CachePadded<UnsafeCell<T>>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            next_ticket: CachePadded::new(AtomicUsize::new(0)),
            now_serving: CachePadded::new(AtomicUsize::new(0)),
            value: CachePadded::new(UnsafeCell::new(value)),
        }
    }

    /// Acquire the lock and return the ticket number used to do so.
    unsafe fn acquire(&self) -> usize {
        // Grab the next free ticket by incrementing the ticket counter.
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        // Wait until it's our turn by continously checking the ticket number.
        // A relaxed ordering would also work here but better performance
        // has been observed on weak architectures with an acquire load.
        while self.now_serving.load(Ordering::Acquire) != ticket {
            spin_loop_hint();
        }

        ticket
    }

    /// This function moves the current thread out of the line and increments the ticket number.
    /// We can use a store instead of a CAS here because only one thread may execute this function
    /// at any given time since it may only be executed while holding the lock.
    unsafe fn release(&self, ticket: usize) {
        let next_ticket = ticket.wrapping_add(1);
        self.now_serving.store(next_ticket, Ordering::Release);
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
