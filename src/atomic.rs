use crate::{ReclaimableManager, Reclaimer, Shared, Tag};
use std::{
    marker::PhantomData,
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Atomic<R, M, T>
where
    R: Reclaimer<M>,
    M: ReclaimableManager,
    T: Tag,
{
    data: AtomicUsize,
    _m0: PhantomData<M>,
    _m1: PhantomData<R>,
    _m2: PhantomData<T>,
}

impl<R, M, T> Atomic<R, M, T>
where
    R: Reclaimer<M>,
    M: ReclaimableManager,
    T: Tag,
{
    pub fn null() -> Self {
        Self::from_data(0)
    }

    pub fn null_vec(len: usize) -> Vec<Self> {
        let unsync_vec = vec![0; len];
        unsafe { mem::transmute(unsync_vec) }
    }

    pub fn from_ptr(ptr: *mut M::Reclaimable) -> Self {
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
