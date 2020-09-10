use std::{
    cell::UnsafeCell,
    cmp,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

const BUFFER_SIZE: usize = 32;

pub struct DrainQueue<T> {
    head: AtomicPtr<Node<T>>,
    len: AtomicUsize,
}

impl<T> DrainQueue<T> {
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            len: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, item: T) {
        loop {
            let lnode = self.head.load(Ordering::SeqCst);

            if lnode.is_null() {
                let new_node = Node::new_boxed(ptr::null_mut());

                if self
                    .head
                    .compare_exchange_weak(lnode, new_node, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    unsafe {
                        Box::from_raw(new_node);
                    }
                }

                continue;
            }

            let lnode_ref = unsafe { &mut *lnode };
            let idx = lnode_ref.enqidx.fetch_add(1, Ordering::SeqCst);

            if idx > BUFFER_SIZE - 1 {
                if self.head.load(Ordering::SeqCst) != lnode {
                    continue;
                }

                let new_node = Node::new_boxed(lnode);

                if self
                    .head
                    .compare_exchange_weak(lnode, new_node, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    unsafe {
                        Box::from_raw(new_node);
                    }
                }

                continue;
            }

            unsafe {
                let item_ptr = (*lnode_ref.buffer[idx].as_ptr()).get();
                ptr::write(item_ptr, item);
                self.len.fetch_add(1, Ordering::SeqCst);
                break;
            }
        }
    }

    pub fn swap_out(&self) -> Self {
        let head = self.head.swap(ptr::null_mut(), Ordering::SeqCst);
        let len = self.len.swap(0, Ordering::SeqCst);

        Self {
            head: AtomicPtr::new(head),
            len: AtomicUsize::new(len),
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let lnode = self.head.load(Ordering::SeqCst);

        if lnode.is_null() {
            return None;
        }

        let lnode_ref = unsafe { &mut *lnode };
        let idx_ref = lnode_ref.enqidx.get_mut();
        *idx_ref = cmp::min(*idx_ref, BUFFER_SIZE - 1);
        *idx_ref -= 1;
        let idx = *idx_ref;

        unsafe {
            let item_ptr = (*lnode_ref.buffer[idx].as_ptr()).get();
            let value = ptr::read(item_ptr);

            if idx == 0 {
                *self.head.get_mut() = *lnode_ref.next.get_mut();
                Box::from_raw(lnode);
            }

            self.len.fetch_sub(1, Ordering::SeqCst);
            Some(value)
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::SeqCst)
    }
}

impl<T> Drop for DrainQueue<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

struct Node<T> {
    next: AtomicPtr<Self>,
    enqidx: AtomicUsize,
    buffer: [MaybeUninit<UnsafeCell<T>>; BUFFER_SIZE],
}

impl<T> Node<T> {
    fn new(next: *mut Self) -> Self {
        Self {
            next: AtomicPtr::new(next),
            enqidx: AtomicUsize::new(0),
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    fn new_boxed(next: *mut Self) -> *mut Self {
        Box::into_raw(Box::new(Self::new(next)))
    }
}
