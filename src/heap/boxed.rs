use crate::alloc::{AllocRef, Layout};
use core::{mem, mem::MaybeUninit, ops::Deref, ptr};

unsafe fn assume_init_read<T>(v: &MaybeUninit<T>) -> T {
    ptr::read(v.as_ptr())
}

pub struct Box<T> {
    allocator: MaybeUninit<AllocRef>,
    raw: *mut T,
}

impl<T> Box<T> {
    pub fn new(value: T, allocator: AllocRef) -> Self {
        let layout = Layout::new::<T>();
        let raw = allocator.alloc(&layout) as *mut T;

        unsafe {
            ptr::write(raw, value);
        }

        Self {
            allocator: MaybeUninit::new(allocator),
            raw,
        }
    }

    pub fn move_out(self) -> T {
        let layout = Layout::new::<T>();

        unsafe {
            let value = ptr::read(self.raw);
            assume_init_read(&self.allocator).dealloc(&layout, self.raw as *mut u8);
            mem::forget(self);
            value
        }
    }

    pub fn into_raw(self) -> (*mut T, AllocRef) {
        let allocator = unsafe { assume_init_read(&self.allocator) };
        let raw = self.raw;
        mem::forget(self);
        (raw, allocator)
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
            assume_init_read(&self.allocator).dealloc(&layout, self.raw as *mut u8);
        }
    }
}

unsafe impl<T> Send for Box<T> where T: Send {}
unsafe impl<T> Sync for Box<T> where T: Sync {}
