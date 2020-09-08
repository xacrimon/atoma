use std::{
    iter,
    mem,
    ptr,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

struct Node<T> {
    next: AtomicPtr<Self>,
    value: T,
}

pub struct Queue<T> {
    head: AtomicPtr<Node<T>>,
    len: AtomicUsize,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            len: AtomicUsize::new(0)
        }
    }

    pub fn push(&self, value: T) {
        let mut node = Box::new(Node {
            next: AtomicPtr::new(ptr::null_mut()),
            value,
        });

        let node_ptr = &*node as *const Node<T> as *mut Node<T>;

        loop {
            let prev_head = self.head.load(Ordering::Acquire);
            *node.next.get_mut() = prev_head;

            if self
                .head
                .compare_and_swap(prev_head, node_ptr, Ordering::AcqRel)
                == prev_head
            {
                self.len.fetch_add(1, Ordering::AcqRel);
                mem::forget(node);
                break;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a {
        let mut next = self.head.load(Ordering::Acquire);

        iter::from_fn(move || {
            if next.is_null() {
                None
            } else {
                let node = unsafe { &*next };
                next = node.next.load(Ordering::Acquire);
                Some(&node.value)
            }
        })
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        let mut next = *self.head.get_mut();

        while !next.is_null() {
            let node = unsafe { &mut *next };
            next = *node.next.get_mut();
            unsafe { Box::from_raw(node) };
        }
    }
}
