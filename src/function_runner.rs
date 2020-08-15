use crate::ReclaimableManager;
use std::{
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr,
};

pub struct FunctionRunner;

impl ReclaimableManager for FunctionRunner {
    type Reclaimable = Function;

    fn reclaim(&self, object: Self::Reclaimable) {
        object.call();
    }
}

const DATA_SIZE: usize = 12;
type Data = [u8; DATA_SIZE];

pub struct Function {
    call: unsafe fn(*mut u8),
    data: Data,
    _m0: PhantomData<*mut ()>,
}

impl Function {
    pub fn new<F: FnOnce()>(f: F) -> Self {
        let size = mem::size_of::<F>();
        let align = mem::align_of::<F>();

        unsafe {
            if size <= mem::size_of::<Data>() && align <= mem::align_of::<Data>() {
                let mut data = MaybeUninit::<Data>::uninit();
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
                let b: Box<F> = Box::new(f);
                let mut data = MaybeUninit::<Data>::uninit();
                ptr::write(data.as_mut_ptr() as *mut Box<F>, b);

                unsafe fn call<F: FnOnce()>(raw: *mut u8) {
                    let b: Box<F> = ptr::read(raw as *mut Box<F>);
                    (*b)();
                }

                Self {
                    call: call::<F>,
                    data: data.assume_init(),
                    _m0: PhantomData,
                }
            }
        }
    }

    pub fn call(mut self) {
        unsafe { (self.call)(&mut self.data as *mut Data as *mut u8) }
    }
}
