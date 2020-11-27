mod priority_queue;
mod thread_id;

use std::{
    marker::PhantomData,
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};

const MAX_THREADS: usize = 1024;

pub struct ThreadLocal<T> {
    entries: Box<[AtomicUsize; MAX_THREADS]>,
    snapshot: AtomicUsize,
    _m0: PhantomData<*mut T>,
}

impl<T> ThreadLocal<T> {
    pub fn new() -> Self {
        let arr = unsafe { mem::transmute([0_usize; MAX_THREADS]) };

        Self {
            entries: Box::new(arr),
            snapshot: AtomicUsize::new(0),
            _m0: PhantomData,
        }
    }

    pub fn get<F>(&self, create: F) -> &T
    where
        F: FnOnce() -> T,
    {
        let id = thread_id::get();
        let entry = self.entries[id].load(Ordering::SeqCst);

        if entry == 0 {
            self.snapshot.fetch_add(1, Ordering::SeqCst);
            let item = Box::new(create());
            let raw = Box::into_raw(item) as usize;
            self.entries[id].store(raw, Ordering::SeqCst);
            self.snapshot.fetch_add(1, Ordering::SeqCst);
            unsafe { &*(raw as *const T) }
        } else {
            unsafe { &*(entry as *const T) }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.entries
            .iter()
            .filter_map(|atomic| unsafe { (atomic.load(Ordering::SeqCst) as *const T).as_ref() })
    }

    pub fn snapshot(&self) -> Snapshot {
        let snapshot = self.snapshot.load(Ordering::SeqCst);
        Snapshot(snapshot)
    }

    pub fn changed_since(&self, snapshot: Snapshot) -> bool {
        !(self.snapshot.load(Ordering::SeqCst) == snapshot.0)
    }
}

unsafe impl<T> Send for ThreadLocal<T> where T: Send {}
unsafe impl<T> Sync for ThreadLocal<T> where T: Sync {}

pub struct Snapshot(usize);
