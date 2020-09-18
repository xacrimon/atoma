use super::thread_id;
use std::{
    mem, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

/// A wait-free table mapping thread ids to pointers.
/// Because we try to keep thread ids low and reuse them
/// this is implemented as a lookup table instead of a hash table.
/// To allow incremental resizing we also store the previous table if any.
pub struct Table<T> {
    pub(crate) buckets: Box<[AtomicPtr<T>]>,
    pub previous: Option<Box<Self>>,
}

impl<T> Table<T> {
    #[cold]
    #[inline(never)]
    pub fn new(max: usize, previous: Option<Box<Self>>) -> Self {
        fn init_empty_buckets<T>(amount: usize) -> Box<[AtomicPtr<T>]> {
            let unsync_buckets = vec![ptr::null_mut::<T>(); amount].into_boxed_slice();
            unsafe { mem::transmute(unsync_buckets) }
        }

        Self {
            buckets: init_empty_buckets(max),
            previous,
        }
    }

    /// Get the numerically largest thread id this table can store.
    #[inline]
    pub fn max_id(&self) -> usize {
        self.buckets.len() - 1
    }

    /// # Safety
    /// - `key` must be below or equal to `self.max_id()`
    /// - `key` must be the id of the calling thread
    #[inline]
    pub unsafe fn get_as_owner(&self, key: usize) -> Option<*mut T> {
        debug_assert_eq!(key, thread_id::get() as usize);
        self.get(key, Ordering::Relaxed)
    }

    /// # Safety
    /// - `key` must be below or equal to `self.max_id()`
    #[inline]
    unsafe fn get(&self, key: usize, order: Ordering) -> Option<*mut T> {
        debug_assert!(key <= self.max_id());
        let ptr = self.buckets.get_unchecked(key).load(order);

        // empty buckets are represented as null
        if !ptr.is_null() {
            Some(ptr)
        } else {
            None
        }
    }

    /// # Safety
    /// - `key` must be below or equal to `self.max_id()`
    /// - `key` must be the id of the calling thread
    /// - `key` must not have been set earlier
    #[cold]
    #[inline(never)]
    pub unsafe fn set(&self, key: usize, ptr: *mut T) {
        debug_assert!(key <= self.max_id());
        debug_assert_eq!(key, thread_id::get() as usize);
        let atomic = self.buckets.get_unchecked(key);

        #[cfg(debug_assertions)]
        {
            let old = atomic.compare_and_swap(ptr::null_mut(), ptr, Ordering::Release);
            debug_assert!(old.is_null());
        }

        #[cfg(not(debug_assertions))]
        atomic.store(ptr, Ordering::Release);
    }

    #[inline]
    pub fn previous(&self) -> Option<&Self> {
        self.previous.as_deref()
    }

    #[cold]
    #[inline(never)]
    pub unsafe fn drop_manual(&mut self, freed_set: &mut Vec<*mut T>) {
        if let Some(mut previous) = self.previous.take() {
            previous.drop_manual(freed_set);
        }

        for atomic_ptr in &*self.buckets {
            let ptr = atomic_ptr.load(Ordering::Acquire);

            // create a box from the pointer and drop it if it isn't null
            if !ptr.is_null() && !freed_set.contains(&ptr) {
                // if it isn't null `ptr` must be pointing to a valid table
                freed_set.push(ptr);
                Box::from_raw(ptr);
            }
        }
    }
}
