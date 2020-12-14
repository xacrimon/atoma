use crate::alloc::{AllocRef, Layout};
use core::{
    cell::UnsafeCell,
    ops::Deref,
    ptr,
    sync::atomic::{AtomicIsize, Ordering},
};

pub struct Arc<T> {
    state: *mut ArcState<T>,
}

impl<T> Arc<T> {
    pub fn new(value: T, allocator: AllocRef) -> Self {
        let layout = Layout::new::<ArcState<T>>();
        let ptr = allocator.alloc(&layout) as *mut ArcState<T>;

        let state = ArcState {
            refs: AtomicIsize::new(1),
            allocator,
            data: UnsafeCell::new(value),
        };

        unsafe {
            ptr::write(ptr, state);
        }

        Self { state: ptr }
    }

    fn refs_mod(&self, x: isize) -> isize {
        self.state().refs.fetch_add(1, Ordering::SeqCst) + x
    }

    fn state(&self) -> &ArcState<T> {
        unsafe { &*self.state }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.state().data.get() }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        self.refs_mod(1);
        Self { state: self.state }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.refs_mod(-1) == 0 {
            let layout = Layout::new::<ArcState<T>>();

            unsafe {
                let allocator = self.state().allocator.clone();
                ptr::drop_in_place(self.state);
                allocator.dealloc(&layout, self.state as *mut u8);
            }
        }
    }
}

unsafe impl<T> Send for Arc<T> where T: Send {}
unsafe impl<T> Sync for Arc<T> where T: Sync {}

#[repr(C)]
struct ArcState<T> {
    data: UnsafeCell<T>,
    allocator: AllocRef,
    refs: AtomicIsize,
}
