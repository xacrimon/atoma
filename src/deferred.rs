use core::{
    mem::{self, MaybeUninit},
    ptr,
};

type Data = [usize; 4];

/// A `Deferred` is a concrete type that stores a closure implementing `FnOnce()`.
/// This type has one primary advantage over simply boxing the closure. When
/// the closures associated capture data struct is less than 4 words.
/// the closure is stored fully inline without any sort of allocation.
/// Should it exceed some amount of words it will act as a boxed closure.
pub struct Deferred {
    // call stores the specialized function that will extract and read the closure.
    call: unsafe fn(*mut u8),

    // data is either a type-erased closure or a pointer to the boxed closure.
    data: MaybeUninit<Data>,
}

impl Deferred {
    pub fn new<F: FnOnce() + 'static>(f:F) -> Self {
        unsafe {
            if mem::size_of::<F>() <= mem::size_of::<Data>() && mem::align_of::<F>() <= mem::align_of::<Data>() {
                // Store the closure inline if it fits.
                let mut data = MaybeUninit::<Data>::uninit();

                #[allow(clippy::cast_ptr_alignment)]
                ptr::write(data.as_mut_ptr() as *mut F, f);

                unsafe fn call<F: FnOnce()>(raw: *mut u8) {
                    let f: F = ptr::read(raw as *mut F);
                    f();
                }

                Self {
                    call: call::<F>,
                    data,
                }
            } else {
                // The closure was too large, let's box it instead.
                let b: Box<F> = Box::new(f);
                let mut data = MaybeUninit::<Data>::uninit();

                #[allow(clippy::cast_ptr_alignment)]
                ptr::write(data.as_mut_ptr() as *mut Box<F>, b);

                unsafe fn call<F: FnOnce()>(raw: *mut u8) {
                    #[allow(clippy::cast_ptr_alignment)]
                    let b: F = Box::into_inner(ptr::read(raw as *mut Box<F>));
                    b();
                }

                Self {
                    call: call::<F>,
                    data,
                }
            }
        }
    }

    pub fn call(mut self) {
        unsafe { (self.call)(self.data.as_mut_ptr() as *mut u8) }
    }
}

#[cfg(test)]
mod tests {
    use super::{Deferred, Data};
    use std::sync::atomic::{self, AtomicI32, AtomicPtr};
    use std::mem;
    use std::ptr;

    #[test]
    fn run_small_closure() {
        static COUNTER: AtomicI32 = AtomicI32::new(0);
        let closure = || {COUNTER.fetch_add(1, atomic::Ordering::SeqCst);};

        assert!(mem::size_of_val(&closure) <= mem::size_of::<Data>());
        assert!(mem::align_of_val(&closure) <= mem::size_of::<Data>());

        let deferred = Deferred::new(closure);
        deferred.call();

        assert!(COUNTER.load(atomic::Ordering::SeqCst) == 1);
    }

    #[test]
    fn run_large_closure() {
        static COUNTER: AtomicI32 = AtomicI32::new(0);
        static PTR: AtomicPtr<[u8; 1024]> = AtomicPtr::new(ptr::null_mut());

        let arr = [0_u8; 1024];
        let closure = move || {
            let ptr = Box::leak(Box::new(arr));
            PTR.store(ptr, atomic::Ordering::SeqCst);
            COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        };

        assert!(mem::size_of_val(&closure) > mem::size_of::<Data>());

        let deferred = Deferred::new(closure);
        deferred.call();

        assert!(COUNTER.load(atomic::Ordering::SeqCst) == 1);
    }
}
