use crate::{alloc::AllocRef, heap::Box};
use core::{
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr,
};

const DATA_SIZE: usize = 3;
type Data = [usize; DATA_SIZE];

// note to a future reader
// this module is a giant clusterfuck and I have no idea if half of this is legal
// but I pray it is and it doesn't segfault so I'll assume I can ship it
//
// please consider opening an issue if you find something you think isn't legal to do

/// A `Deferred` is a concrete type that stores a closure implementing `FnOnce()`.
/// This type has one primary advantage over simply boxing the closure. When
/// the closures associated capture data struct is less than 3 words.
/// the closure is stored fully inline without any sort of allocation.
/// Should it exceed some amount of words it will act as a boxed closure.
pub struct Deferred {
    call: unsafe fn(*mut u8),
    data: Data,
    _m0: PhantomData<*mut ()>,
}

impl Deferred {
    pub fn new<F: FnOnce()>(f: F, allocator: &AllocRef) -> Self {
        let size = mem::size_of::<F>();
        let align = mem::align_of::<F>();

        unsafe {
            if size <= mem::size_of::<Data>() && align <= mem::align_of::<Data>() {
                // store it inline if it fits
                let mut data = MaybeUninit::<Data>::uninit();

                // I pray this is also safe, otherwise we're in trouble.
                #[allow(clippy::cast_ptr_alignment)]
                ptr::write(data.as_mut_ptr() as *mut F, f);

                unsafe fn call<F: FnOnce()>(raw: *mut u8) {
                    let f: F = ptr::read(raw as *mut F);
                    f();
                }

                Self {
                    call: call::<F>,
                    data: data.assume_init(),
                    _m0: PhantomData,
                }
            } else {
                // box it instead
                let b: Box<F> = Box::new(f, allocator.clone());
                let mut data = MaybeUninit::<Data>::uninit();

                // this should be safe but another pair of eyes wouldn't hurt
                #[allow(clippy::cast_ptr_alignment)]
                ptr::write(data.as_mut_ptr() as *mut Box<F>, b);

                unsafe fn call<F: FnOnce()>(raw: *mut u8) {
                    #[allow(clippy::cast_ptr_alignment)]
                    let b: F = ptr::read(raw as *mut Box<F>).move_out();
                    b();
                }

                Self {
                    call: call::<F>,
                    data: data.assume_init(),
                    _m0: PhantomData,
                }
            }
        }
    }

    fn empty() -> Self {
        unsafe fn call(_: *mut u8) {}

        Self {
            call,
            data: [0; DATA_SIZE],
            _m0: PhantomData,
        }
    }

    pub fn call(mut self) {
        unsafe { (self.call)(&mut self.data as *mut Data as *mut u8) }
    }
}

impl Default for Deferred {
    fn default() -> Self {
        Self::empty()
    }
}
