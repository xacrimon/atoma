//! Example implementation of FAAArrayQueue using flize.
//! Reference: http://concurrencyfreaks.blogspot.com/2016/11/faaarrayqueue-mpmc-lock-free-queue-part.html

use flize::{unprotected, Atomic, Collector, Shared, Shield};
use std::mem;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

const BUFFER_SIZE: usize = 1024;

pub struct Queue {
    collector: Collector,
    head: Atomic<Node>,
    tail: Atomic<Node>,
}

impl Queue {
    fn cas_tail<'s, S>(&self, current: Shared<Node>, new: Shared<Node>, shield: &S) -> bool
    where
        S: Shield<'s>,
    {
        self.tail
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst, shield)
            .is_ok()
    }

    fn cas_head<'s, S>(&self, current: Shared<Node>, new: Shared<Node>, shield: &S) -> bool
    where
        S: Shield<'s>,
    {
        self.head
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst, shield)
            .is_ok()
    }

    pub fn new() -> Self {
        let sentinel = Node::new(0, true);

        Self {
            collector: Collector::new(),
            head: Atomic::new(sentinel),
            tail: Atomic::new(sentinel),
        }
    }

    pub fn push(&self, value: NonZeroU64) {
        let shield = self.collector.thin_shield();

        loop {
            let ltail = self.tail.load(Ordering::SeqCst, &shield);
            let ltailr = unsafe { ltail.as_ref_unchecked() };
            let idx = ltailr.enqidx.fetch_add(1, Ordering::SeqCst);

            if idx > BUFFER_SIZE - 1 {
                if ltail != self.tail.load(Ordering::SeqCst, &shield) {
                    continue;
                }

                let lnext = ltailr.next.load(Ordering::SeqCst, &shield);

                if lnext.is_null() {
                    let new_node = Node::new(value.get(), false);

                    if ltailr.cas_next(Shared::null(), new_node, &shield) {
                        self.cas_tail(ltail, new_node, &shield);
                        return;
                    }

                    unsafe {
                        Box::from_raw(new_node.as_ptr());
                    }
                } else {
                    self.cas_tail(ltail, lnext, &shield);
                }
            } else if ltailr.items[idx]
                .compare_exchange(0, value.get(), Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return;
            }
        }
    }

    pub fn pop(&self) -> Option<NonZeroU64> {
        let shield = self.collector.thin_shield();

        loop {
            let lhead = self.head.load(Ordering::SeqCst, &shield);
            let lheadr = unsafe { lhead.as_ref_unchecked() };

            if lheadr.deqidx.load(Ordering::SeqCst) >= lheadr.enqidx.load(Ordering::SeqCst)
                && lheadr.next.load(Ordering::SeqCst, &shield).is_null()
            {
                break None;
            }

            let idx = lheadr.deqidx.fetch_add(1, Ordering::SeqCst);

            if idx > BUFFER_SIZE - 1 {
                let lnext = lheadr.next.load(Ordering::SeqCst, &shield);

                if lnext.is_null() {
                    break None;
                }

                if self.cas_head(lhead, lnext, &shield) {
                    shield.retire(move || unsafe {
                        Box::from_raw(lhead.as_ptr());
                    });
                }

                continue;
            }

            let item = lheadr.items[idx].swap(0, Ordering::SeqCst);

            if item == 0 {
                continue;
            }

            return Some(unsafe { NonZeroU64::new_unchecked(item) });
        }
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}

        unsafe {
            let shield = unprotected();
            let lhead = self.head.load(Ordering::SeqCst, shield);
            Box::from_raw(lhead.as_ptr());
        }
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

struct Node {
    enqidx: AtomicUsize,
    deqidx: AtomicUsize,
    items: [AtomicU64; BUFFER_SIZE],
    next: Atomic<Self>,
}

impl Node {
    fn new(first: u64, sentinel: bool) -> Shared<'static, Self> {
        let start_enq = if sentinel { 0 } else { 1 };
        let mut items: [AtomicU64; BUFFER_SIZE] = unsafe { mem::transmute([0_u64; BUFFER_SIZE]) };
        *items[0].get_mut() = first;

        let raw = Box::into_raw(Box::new(Self {
            enqidx: AtomicUsize::new(start_enq),
            deqidx: AtomicUsize::new(start_enq),
            items,
            next: Atomic::null(),
        }));

        unsafe { Shared::from_ptr(raw) }
    }

    fn cas_next<'s, S>(&self, current: Shared<Self>, new: Shared<Self>, shield: &S) -> bool
    where
        S: Shield<'s>,
    {
        self.next
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst, shield)
            .is_ok()
    }
}
