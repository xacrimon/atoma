mod priority_queue;
mod thread_id;

pub use thread_id::{ThreadId, TlsProvider};

#[cfg(feature = "std")]
pub use thread_id::std_tls_provider;

use crate::{alloc::AllocRef, heap::Box};
use core::{
    marker::PhantomData,
    mem,
    sync::atomic::{self, AtomicUsize, Ordering},
};

const MAX_THREADS: usize = 1024;

pub(crate) struct ThreadLocal<T> {
    entries: Box<[AtomicUsize; MAX_THREADS]>,
    snapshot: AtomicUsize,
    tls_provider: &'static dyn TlsProvider,
    _m0: PhantomData<*mut T>,
    allocator: AllocRef,
}

impl<T> ThreadLocal<T> {
    pub fn new(tls_provider: &'static dyn TlsProvider, allocator: AllocRef) -> Self {
        let arr = unsafe { mem::transmute([0_usize; MAX_THREADS]) };

        Self {
            entries: Box::new(arr, allocator.clone()),
            snapshot: AtomicUsize::new(0),
            tls_provider,
            _m0: PhantomData,
            allocator,
        }
    }

    pub fn get<F>(&self, create: F) -> &T
    where
        F: FnOnce() -> T,
    {
        let id = self.tls_provider.get();
        let entry = unsafe { self.entries.get_unchecked(id).load(Ordering::Relaxed) };

        if entry == 0 {
            self.snapshot.fetch_add(1, Ordering::Release);
            atomic::compiler_fence(Ordering::SeqCst);
            let item = Box::new(create(), self.allocator.clone());
            let raw = Box::into_raw(item).0 as usize;
            unsafe {
                self.entries.get_unchecked(id).store(raw, Ordering::Release);
            }
            atomic::compiler_fence(Ordering::SeqCst);
            self.snapshot.fetch_add(1, Ordering::Release);
            unsafe { &*(raw as *const T) }
        } else {
            unsafe { &*(entry as *const T) }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.entries
            .iter()
            .filter_map(|atomic| unsafe { (atomic.load(Ordering::Acquire) as *const T).as_ref() })
    }

    pub fn snapshot(&self) -> Snapshot {
        let snapshot = self.snapshot.load(Ordering::Acquire);
        Snapshot(snapshot)
    }

    pub fn changed_since(&self, snapshot: Snapshot) -> bool {
        self.snapshot.load(Ordering::Acquire) != snapshot.0
    }
}

unsafe impl<T> Send for ThreadLocal<T> where T: Send {}
unsafe impl<T> Sync for ThreadLocal<T> where T: Sync {}

pub(crate) struct Snapshot(usize);
