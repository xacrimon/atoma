use crate::{Shared, Shield, Tag};
use std::{
    marker::PhantomData,
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};

fn map_both<T, U, F>(result: Result<T, T>, f: F) -> Result<U, U>
where
    F: FnOnce(T) -> U,
{
    match result {
        Ok(v) => Ok(f(v)),
        Err(v) => Err(f(v)),
    }
}

#[repr(transparent)]
pub struct Atomic<V, T>
where
    T: Tag,
{
    pub(crate) data: AtomicUsize,
    _m0: PhantomData<V>,
    _m1: PhantomData<T>,
}

impl<V, T> Atomic<V, T>
where
    T: Tag,
{
    pub unsafe fn from_raw(raw: usize) -> Self {
        Self {
            data: AtomicUsize::new(raw),
            _m0: PhantomData,
            _m1: PhantomData,
        }
    }

    pub fn null() -> Self {
        unsafe { Self::from_raw(0) }
    }

    pub fn null_vec(len: usize) -> Vec<Self> {
        unsafe { mem::transmute(vec![0; len]) }
    }

    pub fn load<'shield>(
        &self,
        ordering: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Shared<'shield, V, T> {
        let raw = self.data.load(ordering);
        unsafe { Shared::from_raw(raw) }
    }

    pub fn store<'shield>(
        &self,
        data: Shared<'_, V, T>,
        ordering: Ordering,
        _shield: &'shield Shield<'_>,
    ) {
        let raw = data.into_raw();
        self.data.store(raw, ordering);
    }

    pub fn swap<'shield>(
        &self,
        new: Shared<'_, V, T>,
        ordering: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Shared<'shield, V, T> {
        let new_raw = new.into_raw();
        let old_raw = self.data.swap(new_raw, ordering);
        unsafe { Shared::from_raw(old_raw) }
    }

    pub fn compare_and_swap<'shield>(
        &self,
        current: Shared<'_, V, T>,
        new: Shared<'_, V, T>,
        order: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Shared<'shield, V, T> {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        let old_raw = self.data.compare_and_swap(current_raw, new_raw, order);
        unsafe { Shared::from_raw(old_raw) }
    }

    pub fn compare_exchange_weak<'shield>(
        &self,
        current: Shared<'_, V, T>,
        new: Shared<'_, V, T>,
        success: Ordering,
        failure: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Result<Shared<'shield, V, T>, Shared<'shield, V, T>> {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        let result = self
            .data
            .compare_exchange_weak(current_raw, new_raw, success, failure);

        map_both(result, |raw| unsafe { Shared::from_raw(raw) })
    }
}
