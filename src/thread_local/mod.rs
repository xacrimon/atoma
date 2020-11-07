mod priority_queue;
mod table;
mod thread_id;

use core::{
    marker::PhantomData,
    sync::atomic::{fence, AtomicPtr, AtomicUsize, Ordering},
};
use table::Table;

/// A wrapper that keeps different instances of something per thread.
///
/// Think of this as a non global thread-local variable.
/// Threads may occasionally get an old value that another thread previously had.
/// There isn't a nice way to avoid this without compromising on performance.
pub struct ThreadLocal<T: Send + Sync> {
    table: AtomicPtr<Table<T>>,
    len: AtomicUsize,
    mod_acc: AtomicUsize,
}

impl<T: Send + Sync> ThreadLocal<T> {
    pub fn new() -> Self {
        let table = Table::new(4, None);
        let table_ptr = Box::into_raw(Box::new(table));

        Self {
            table: AtomicPtr::new(table_ptr),
            len: AtomicUsize::new(0),
            mod_acc: AtomicUsize::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::SeqCst)
    }

    pub fn mod_acc(&self) -> usize {
        self.mod_acc.load(Ordering::SeqCst)
    }

    /// Get the value for this thread or initialize it with the given function if it doesn't exist.
    pub fn get<F>(&self, create: F) -> &T
    where
        F: FnOnce() -> T,
    {
        let id = thread_id::get();
        let id_usize = id as usize;

        self.get_fast(id_usize).unwrap_or_else(|| {
            let data = Box::into_raw(Box::new(create()));
            unsafe { self.insert(id_usize, data, true) }
        })
    }

    /// Iterate over values.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            remaining: self.len.load(Ordering::Acquire),
            index: 0,
            table: self.table(Ordering::Acquire),
            _m0: PhantomData,
        }
    }

    fn table(&self, order: Ordering) -> &Table<T> {
        unsafe { &*self.table.load(order) }
    }

    // Fast path, checks the top level table.
    fn get_fast(&self, key: usize) -> Option<&T> {
        let table = self.table(Ordering::Relaxed);

        if key > table.max_id() {
            None
        } else {
            match unsafe { table.get_as_owner(key) } {
                Some(x) => Some(unsafe { &*x }),
                None => self.get_slow(key),
            }
        }
    }

    /// Slow path, searches tables recursively.
    fn get_slow(&self, key: usize) -> Option<&T> {
        let mut current = Some(self.table(Ordering::Acquire));

        while let Some(table) = current {
            if key <= table.max_id() {
                if let Some(x) = unsafe { table.get_as_owner(key) } {
                    return Some(unsafe { self.insert(key, x, false) });
                }
            }

            current = table.previous();
        }

        None
    }

    /// Insert into the top level table.
    ///
    /// # Safety
    /// A key may not be inserted two times with the same top level table.
    unsafe fn insert(&self, key: usize, data: *mut T, new: bool) -> &T {
        self.mod_acc.fetch_add(1, Ordering::SeqCst);

        loop {
            let table = self.table(Ordering::Acquire);

            let actual_table = if key > table.max_id() {
                let old_table_ptr = table as *const Table<T> as *mut Table<T>;
                let old_table = Box::from_raw(old_table_ptr);
                let new_table = Table::new(key * 2, Some(old_table));
                let new_table_ptr = Box::into_raw(Box::new(new_table));

                if self
                    .table
                    .compare_exchange_weak(
                        old_table_ptr,
                        new_table_ptr,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    &*new_table_ptr
                } else {
                    continue;
                }
            } else {
                table
            };

            actual_table.set(key, data);

            if new {
                self.len.fetch_add(1, Ordering::Release);
            }

            self.mod_acc.fetch_add(1, Ordering::SeqCst);
            fence(Ordering::SeqCst);
            break &*data;
        }
    }
}

pub struct Iter<'a, T> {
    remaining: usize,
    index: usize,
    table: *const Table<T>,
    _m0: PhantomData<&'a ()>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        };

        loop {
            let table = unsafe { &*self.table };
            let entries = &table.buckets;

            while self.index < entries.len() {
                let val = entries[self.index].load(Ordering::Acquire);
                self.index += 1;

                if !val.is_null() {
                    self.remaining -= 1;
                    return unsafe { Some(&*val) };
                }
            }

            self.index = 0;
            self.table = table.previous().unwrap();
        }
    }
}

impl<T: Send + Sync> Drop for ThreadLocal<T> {
    fn drop(&mut self) {
        let table_ptr = self.table.load(Ordering::Acquire);

        // the table must always be valid, this drops it and its child tables.
        unsafe {
            let mut freed_set = Vec::new();
            (*table_ptr).drop_manual(&mut freed_set);
            Box::from_raw(table_ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::thread_id;
    use super::ThreadLocal;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
    };

    #[test]
    fn create_insert_store_single_thread() {
        let thread_local = ThreadLocal::new();
        let x = thread_local.get(|| AtomicUsize::new(thread_id::get() as usize));
        x.store(1, Ordering::SeqCst);
    }

    #[test]
    fn create_insert_store_multi_thread() {
        let thread_local = Arc::new(ThreadLocal::new());
        let mut threads = Vec::new();

        for _ in 0..32 {
            let thread_local = Arc::clone(&thread_local);

            threads.push(thread::spawn(move || {
                let atomic_stored_id =
                    thread_local.get(|| AtomicUsize::new(thread_id::get() as usize));
                let stored_id = atomic_stored_id.load(Ordering::SeqCst);
                assert_eq!(stored_id, thread_id::get() as usize);
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }
}
