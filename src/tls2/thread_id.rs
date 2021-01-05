//! This module deals with allocating thread ids.
//! We aggressively reuse ids and try to keep them as low as possible.
//! This important because low reusable thread ids allows us to use lookup tables
//! instead of hash tables for storing thread-local data.

use super::priority_queue::PriorityQueue;
use crate::lazy::Lazy;
use crate::mutex::Mutex;
use core::fmt::Debug;

#[cfg(feature = "std")]
use core::cell::RefCell;

/// A `TlsProvider` is any implementation that can provide storage of a thread id.
/// Because flize is designed to run on systems where the standard library doesn't exist
/// we have this trait so you can use your own implementation.
///
/// The `get` method should return a thread id which this implementation should lazily initialize on the first call.
pub trait TlsProvider: Debug {
    fn get(&self) -> usize;
}

#[cfg(feature = "std")]
/// This provider is only usable with the `std` feature and uses the standard library
/// TLS implemenation.
pub fn std_tls_provider() -> &'static dyn TlsProvider {
    &StdTls
}

#[cfg(feature = "std")]
#[derive(Debug)]
struct StdTls;

#[cfg(feature = "std")]
impl TlsProvider for StdTls {
    fn get(&self) -> usize {
        TLS_VALUE.with(|cell| {
            let mut value = cell.borrow_mut();

            if value.is_none() {
                *value = Some(ThreadId::new());
            }

            value.as_ref().unwrap().0
        })
    }
}

#[cfg(feature = "std")]
thread_local! {
    static TLS_VALUE: RefCell<Option<ThreadId>> = RefCell::new(None);
}

/// This structure allocates ids.
/// It is compose of a `limit` integer and a list of free ids lesser than `limit`.
/// If an allocation is attempted and the list is empty,
/// we increment limit and return the previous value.
struct IdAllocator {
    limit: usize,
    free: PriorityQueue<usize>,
}

impl IdAllocator {
    fn new() -> Self {
        Self {
            limit: 0,
            free: PriorityQueue::new(),
        }
    }

    fn allocate(&mut self) -> usize {
        self.free.pop().unwrap_or_else(|| {
            let id = self.limit;
            self.limit += 1;
            id
        })
    }

    fn deallocate(&mut self, id: usize) {
        self.free.push(id);
    }
}

static ID_ALLOCATOR: Lazy<Mutex<IdAllocator>> = Lazy::new(|| Mutex::new(IdAllocator::new()));

#[derive(Debug)]
/// A thread id acts as a handle and allocation of a thread id. You can create a new instance of this struct
/// to allocate a new thread id in your `TlsProvider` implementation.
///
/// Because this struct acts as a RAII handle that keeps the id allocated you may not drop it until the thread exits.
/// The implementation should take the integer stored in this struct and return it.
pub struct ThreadId(pub usize);

impl ThreadId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(ID_ALLOCATOR.get().lock().allocate())
    }
}

impl Drop for ThreadId {
    fn drop(&mut self) {
        ID_ALLOCATOR.get().lock().deallocate(self.0);
    }
}
