use crate::alloc::{AllocRef, Layout};
use core::{ops::Deref, ptr};

pub struct Box<T> {
    allocator: AllocRef,
    raw: *mut T,
}

impl<T> Box<T> {
    pub fn new(value: T, allocator: AllocRef) -> Self {
        let layout = Layout::new::<T>();
        let raw = allocator.alloc(&layout) as *mut T;

        unsafe {
            ptr::write(raw, value);
        }

        Self { allocator, raw }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.raw }
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        let layout = Layout::new::<T>();

        unsafe {
            ptr::drop_in_place(self.raw);
            self.allocator.dealloc(&layout, self.raw as *mut u8);
        }
    }
}

unsafe impl<T> Send for Box<T> where T: Send {}
unsafe impl<T> Sync for Box<T> where T: Sync {}
