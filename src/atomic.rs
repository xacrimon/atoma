use crate::{NullTag, Shared, Shield, Tag};
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
pub struct Atomic<V, T1 = NullTag, T2 = NullTag>
where
    T1: Tag,
    T2: Tag,
{
    pub(crate) data: AtomicUsize,
    _m0: PhantomData<V>,
    _m1: PhantomData<T1>,
    _m2: PhantomData<T2>,
}

impl<V, T1, T2> Atomic<V, T1, T2>
where
    T1: Tag,
    T2: Tag,
{
    /// # Safety
    /// Marked unsafe because this is not usually what the user wants.
    /// `Atomic::null` should be preferred when possible.
    pub unsafe fn from_raw(raw: usize) -> Self {
        Self {
            data: AtomicUsize::new(raw),
            _m0: PhantomData,
            _m1: PhantomData,
            _m2: PhantomData,
        }
    }

    /// # Safety
    /// The alignment of `V` must free up sufficient low bits so that `T` fits.
    pub fn new(shared: Shared<'_, V, T1, T2>) -> Self {
        unsafe { Self::from_raw(shared.into_raw()) }
    }

    pub fn null() -> Self {
        unsafe { Self::from_raw(0) }
    }

    pub fn null_vec(len: usize) -> Vec<Self> {
        unsafe {
            mem::transmute(vec![0_usize; len])
        }
    }

    pub fn load<'shield>(
        &self,
        ordering: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Shared<'shield, V, T1, T2> {
        let raw = self.data.load(ordering);
        unsafe { Shared::from_raw(raw) }
    }

    pub fn store<'shield>(
        &self,
        data: Shared<'_, V, T1, T2>,
        ordering: Ordering,
        _shield: &'shield Shield<'_>,
    ) {
        let raw = data.into_raw();
        self.data.store(raw, ordering);
    }

    pub fn swap<'shield>(
        &self,
        new: Shared<'_, V, T1, T2>,
        ordering: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Shared<'shield, V, T1, T2> {
        let new_raw = new.into_raw();
        let old_raw = self.data.swap(new_raw, ordering);
        unsafe { Shared::from_raw(old_raw) }
    }

    pub fn compare_and_swap<'shield>(
        &self,
        current: Shared<'_, V, T1, T2>,
        new: Shared<'_, V, T1, T2>,
        order: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Shared<'shield, V, T1, T2> {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        let old_raw = self.data.compare_and_swap(current_raw, new_raw, order);
        unsafe { Shared::from_raw(old_raw) }
    }

    pub fn compare_exchange<'shield>(
        &self,
        current: Shared<'_, V, T1, T2>,
        new: Shared<'_, V, T1, T2>,
        success: Ordering,
        failure: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Result<Shared<'shield, V, T1, T2>, Shared<'shield, V, T1, T2>> {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        let result = self
            .data
            .compare_exchange(current_raw, new_raw, success, failure);

        map_both(result, |raw| unsafe { Shared::from_raw(raw) })
    }

    pub fn compare_exchange_weak<'shield>(
        &self,
        current: Shared<'_, V, T1, T2>,
        new: Shared<'_, V, T1, T2>,
        success: Ordering,
        failure: Ordering,
        _shield: &'shield Shield<'_>,
    ) -> Result<Shared<'shield, V, T1, T2>, Shared<'shield, V, T1, T2>> {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        let result = self
            .data
            .compare_exchange_weak(current_raw, new_raw, success, failure);

        map_both(result, |raw| unsafe { Shared::from_raw(raw) })
    }
}

unsafe impl<V, T1, T2> Send for Atomic<V, T1, T2>
where
    T1: Tag,
    T2: Tag,
{
}

unsafe impl<V, T1, T2> Sync for Atomic<V, T1, T2>
where
    T1: Tag,
    T2: Tag,
{
}

impl<V, T1, T2> Unpin for Atomic<V, T1, T2>
where
    T1: Tag,
    T2: Tag,
{
}
