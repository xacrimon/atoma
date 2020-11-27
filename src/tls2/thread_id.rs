//! This module deals with allocating thread ids.
//! We aggressively reuse ids and try to keep them as low as possible.
//! This important because low reusable thread ids allows us to use lookup tables
//! instead of hash tables for storing thread-local data.

use super::priority_queue::PriorityQueue;
use crate::lazy::Lazy;
use crate::mutex::Mutex;

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

struct ThreadId(usize);

impl ThreadId {
    fn new() -> Self {
        Self(ID_ALLOCATOR.get().lock().allocate())
    }
}

/// Drop is implemented here because it's the only clean way to run code when a thread exits.
impl Drop for ThreadId {
    fn drop(&mut self) {
        ID_ALLOCATOR.get().lock().deallocate(self.0);
    }
}

thread_local! {
    static THREAD_ID: ThreadId = ThreadId::new();
}

pub fn get() -> usize {
    THREAD_ID.with(|data| data.0)
}
