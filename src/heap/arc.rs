use crate::alloc::{AllocRef, Layout};
use crate::mutex::Mutex;
use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ptr;

struct ArcState<T, A> {
    refs: Mutex<i32>,
    data: UnsafeCell<T>,
    allocator: A,
}

impl<T, A> ArcState<T, A>
where
    A: AllocRef,
{
    fn new(data: T, allocator: A) -> *mut Self {
        let layout = Layout::of::<Self>();
        let ptr = unsafe { allocator.alloc(layout) } as *mut Self;

        let instance = Self {
            refs: Mutex::new(0),
            data: UnsafeCell::new(data),
            allocator,
        };

        unsafe {
            ptr::write(ptr, instance);
        }

        ptr
    }

    fn increment(this: *mut Self) {
        unsafe {
            *(*this).refs.lock() += 1;
        }
    }

    fn decrement(this: *mut Self) -> bool {
        let mut refs = unsafe { (*this).refs.lock() };
        *refs -= 1;
        *refs == 0
    }

    unsafe fn deconstruct(this: *mut Self) {
        let layout = Layout::of::<Self>();
        let allocator = (*this).allocator.clone();
        ptr::drop_in_place(this);
        allocator.dealloc(this as *mut u8, layout);
    }

    fn get_shared<'a>(this: *mut Self) -> &'a T {
        unsafe { &*(*this).data.get() }
    }
}

pub struct Arc<T, A>
where
    A: AllocRef,
{
    state: *mut ArcState<T, A>,
}

impl<T, A> Arc<T, A>
where
    A: AllocRef,
{
    pub fn new(value: T, allocator: A) -> Self {
        Self {
            state: ArcState::new(value, allocator),
        }
    }
}

impl<T, A> Deref for Arc<T, A>
where
    A: AllocRef,
{
    type Target = T;

    fn deref(&self) -> &T {
        ArcState::get_shared(self.state)
    }
}

impl<T, A> Clone for Arc<T, A>
where
    A: AllocRef,
{
    fn clone(&self) -> Self {
        ArcState::increment(self.state);
        Self { state: self.state }
    }
}

impl<T, A> Drop for Arc<T, A>
where
    A: AllocRef,
{
    fn drop(&mut self) {
        let should_drop = ArcState::decrement(self.state);

        if should_drop {
            unsafe {
                ArcState::deconstruct(self.state);
            }
        }
    }
}

unsafe impl<T, A> Send for Arc<T, A>
where
    T: Send,
    A: Send + AllocRef,
{
}
unsafe impl<T, A> Sync for Arc<T, A>
where
    T: Sync,
    A: Sync + AllocRef,
{
}
