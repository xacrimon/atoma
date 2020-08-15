use crate::{Reclaimer, Tag};
use std::{marker::PhantomData, mem, sync::atomic::AtomicUsize};

pub struct Atomic<R, V, T>
where
    R: Reclaimer,
    T: Tag,
{
    pub(crate) data: AtomicUsize,
    _m0: PhantomData<R>,
    _m1: PhantomData<V>,
    _m2: PhantomData<T>,
}

impl<R, V, T> Atomic<R, V, T>
where
    R: Reclaimer,
    T: Tag,
{
    pub fn null() -> Self {
        Self::from_data(0)
    }

    pub fn null_vec(len: usize) -> Vec<Self> {
        let unsync_vec = vec![0; len];
        unsafe { mem::transmute(unsync_vec) }
    }

    pub fn from_ptr(ptr: *mut V) -> Self {
        Self::from_data(ptr as usize)
    }

    pub fn from_data(data: usize) -> Self {
        Self {
            data: AtomicUsize::new(data),
            _m0: PhantomData,
            _m1: PhantomData,
            _m2: PhantomData,
        }
    }
}
