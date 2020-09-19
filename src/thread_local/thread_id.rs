//! This module deals with allocating thread ids.
//! We aggressively reuse ids and try to keep them as low as possible.
//! This important because low reusable thread ids allows us to use lookup tables
//! instead of hash tables for storing thread-local data.

use super::priority_queue::PriorityQueue;
use lazy_static::lazy_static;
use std::sync::Mutex;

/// This structure allocates ids.
/// It is compose of a `limit` integer and a list of free ids lesser than `limit`.
/// If an allocation is attempted and the list is empty,
/// we increment limit and return the previous value.
struct IdAllocator {
    limit: u32,
    free: PriorityQueue<u32>,
}

impl IdAllocator {
    #[cold]
    #[inline(never)]
    fn new() -> Self {
        Self {
            limit: 0,
            free: PriorityQueue::new(),
        }
    }

    #[cold]
    #[inline(never)]
    fn allocate(&mut self) -> u32 {
        self.free.pop().unwrap_or_else(|| {
            let id = self.limit;
            self.limit += 1;
            id
        })
    }

    #[cold]
    #[inline(never)]
    fn deallocate(&mut self, id: u32) {
        self.free.push(id);
    }
}

lazy_static! {
    static ref ID_ALLOCATOR: Mutex<IdAllocator> = Mutex::new(IdAllocator::new());
}

struct ThreadId(u32);

impl ThreadId {
    #[cold]
    #[inline(never)]
    fn new() -> Self {
        Self(ID_ALLOCATOR.lock().unwrap().allocate())
    }
}

/// Drop is implemented here because it's the only clean way to run code when a thread exits.
impl Drop for ThreadId {
    #[cold]
    #[inline(never)]
    fn drop(&mut self) {
        ID_ALLOCATOR.lock().unwrap().deallocate(self.0);
    }
}

thread_local! {
    static THREAD_ID: ThreadId = ThreadId::new();
}

#[inline]
pub fn get() -> u32 {
    THREAD_ID.with(|data| data.0)
}
