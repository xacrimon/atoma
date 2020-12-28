use crate::Backoff;
use crate::CachePadded;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{fence, AtomicU8, Ordering};

const DEFAULT_STATE: u8 = 0;
const INIT_MASK: u8 = 1 << 1;
const LOCK_MASK: u8 = 1 << 2;

fn relaxed_is_init(state: &AtomicU8) -> bool {
    state.load(Ordering::Relaxed) & INIT_MASK != 0
}

fn relaxed_set_init(state: &AtomicU8) {
    state.fetch_or(INIT_MASK, Ordering::Relaxed);
}

fn lock_try_acquire(state: &AtomicU8) -> bool {
    fence(Ordering::Acquire);
    state.compare_and_swap(DEFAULT_STATE, LOCK_MASK, Ordering::Relaxed) == DEFAULT_STATE
}

pub struct Lazy<T, F = fn() -> T> {
    state: CachePadded<AtomicU8>,
    value: UnsafeCell<MaybeUninit<T>>,
    init: UnsafeCell<MaybeUninit<F>>,
}

impl<T, F> Lazy<T, F> {
    pub const fn new(init: F) -> Self {
        Self {
            state: CachePadded::new(AtomicU8::new(DEFAULT_STATE)),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            init: UnsafeCell::new(MaybeUninit::new(init)),
        }
    }

    unsafe fn value_ref(&self) -> &T {
        let cell_inner = &*self.value.get();
        let value_ptr = cell_inner.as_ptr();
        &*value_ptr
    }

    unsafe fn write_value(&self, value: T) {
        let cell_ptr = self.value.get();
        ptr::write(cell_ptr, MaybeUninit::new(value));
    }

    unsafe fn load_init(&self) -> F {
        let cell_inner = &*self.init.get();
        let value_ptr = cell_inner.as_ptr();
        ptr::read(value_ptr)
    }
}

impl<T, F> Lazy<T, F>
where
    F: FnOnce() -> T,
{
    pub fn get(&self) -> &T {
        if relaxed_is_init(&self.state) {
            unsafe { self.value_ref() }
        } else {
            self.get_slow()
        }
    }

    fn get_slow(&self) -> &T {
        let backoff = Backoff::new();

        loop {
            fence(Ordering::Acquire);

            if relaxed_is_init(&self.state) {
                break unsafe { self.value_ref() };
            }

            if lock_try_acquire(&self.state) {
                let init = unsafe { self.load_init() };
                let value = init();

                unsafe {
                    self.write_value(value);
                }

                fence(Ordering::Release);
                relaxed_set_init(&self.state);
                break unsafe { self.value_ref() };
            }

            backoff.snooze();
        }
    }
}

unsafe impl<T: Send, F: Send> Send for Lazy<T, F> {}
unsafe impl<T: Sync, F: Send> Sync for Lazy<T, F> {}
