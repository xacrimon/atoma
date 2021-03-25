use crate::{Shared, Shield, Tag};
use core::{
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

fn map_both<T, U, F>(result: Result<T, T>, f: F) -> Result<U, U>
where
    F: FnOnce(T) -> U + Copy,
{
    result.map(f).map_err(f)
}

/// An `Atomic` represents a tagged atomic pointer protected by the collection system.
///
/// This struct provides methods for manipulating the atomic pointer via
/// standard atomic operations using `Shared` as the corresponding non atomic version.
#[repr(transparent)]
pub struct Atomic<V, T1, T2, const N1: usize, const N2: usize>
where
    T1: Tag<N1>,
    T2: Tag<N2>,
{
    pub(crate) data: AtomicUsize,
    _m0: PhantomData<V>,
    _m1: PhantomData<T1>,
    _m2: PhantomData<T2>,
}

impl<V, T1, T2, const N1: usize, const N2: usize> Atomic<V, T1, T2, N1, N2>
where
    T1: Tag<N1>,
    T2: Tag<N2>,
{
    /// Constructs an `Atomic` from a raw tagged pointer represented as an integer.
    ///
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

    /// Constructs a new `Atomic` from a tagged pointer.
    ///
    /// # Safety
    /// The alignment of `V` must free up sufficient low bits so that `T` fits.
    pub fn new(shared: Shared<'_, V, T1, T2, N1, N2>) -> Self {
        unsafe { Self::from_raw(shared.into_raw()) }
    }

    /// Constructs a new `Atomic` with a null value.
    pub fn null() -> Self {
        unsafe { Self::from_raw(0) }
    }

    #[cfg(feature = "std")]
    /// This constructs a `Vec<Atomic>` with null values in an optimized manner.
    pub fn null_vec(len: usize) -> Vec<Self> {
        unsafe { std::mem::transmute(vec![0_usize; len]) }
    }

    /// Load a the tagged pointer.
    pub fn load<'collector, 'shield, S>(
        &self,
        ordering: Ordering,
        _shield: &'shield S,
    ) -> Shared<'shield, V, T1, T2, N1, N2>
    where
        S: Shield<'collector>,
    {
        let raw = self.data.load(ordering);
        unsafe { Shared::from_raw(raw) }
    }

    /// Store a tagged pointer, replacing the previous value.
    pub fn store(&self, data: Shared<'_, V, T1, T2, N1, N2>, ordering: Ordering) {
        let raw = data.into_raw();
        self.data.store(raw, ordering);
    }

    /// Swap the stored tagged pointer, returning the old one.
    pub fn swap<'collector, 'shield, S>(
        &self,
        new: Shared<'_, V, T1, T2, N1, N2>,
        ordering: Ordering,
        _shield: &'shield S,
    ) -> Shared<'shield, V, T1, T2, N1, N2>
    where
        S: Shield<'collector>,
    {
        let new_raw = new.into_raw();
        let old_raw = self.data.swap(new_raw, ordering);
        unsafe { Shared::from_raw(old_raw) }
    }

    /// Conditionally exchange the stored tagged pointer, always returns
    /// the previous value and a result indicating if it was written or not.
    /// On success this value is guaranteed to be equal to current.
    pub fn compare_exchange<'collector, 'shield, S>(
        &self,
        current: Shared<'_, V, T1, T2, N1, N2>,
        new: Shared<'_, V, T1, T2, N1, N2>,
        success: Ordering,
        failure: Ordering,
        _shield: &'shield S,
    ) -> Result<Shared<'shield, V, T1, T2, N1, N2>, Shared<'shield, V, T1, T2, N1, N2>>
    where
        S: Shield<'collector>,
    {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        let result = self
            .data
            .compare_exchange(current_raw, new_raw, success, failure);

        map_both(result, |raw| unsafe { Shared::from_raw(raw) })
    }

    /// Conditionally exchange the stored tagged pointer, always returns
    /// the previous value and a result indicating if it was written or not.
    /// On success this value is guaranteed to be equal to current.
    ///
    /// This variant may spuriously fail on platforms where LL/SC is used.
    /// This allows more efficient code generation on those platforms.
    pub fn compare_exchange_weak<'collector, 'shield, S>(
        &self,
        current: Shared<'_, V, T1, T2, N1, N2>,
        new: Shared<'_, V, T1, T2, N1, N2>,
        success: Ordering,
        failure: Ordering,
        _shield: &'shield S,
    ) -> Result<Shared<'shield, V, T1, T2, N1, N2>, Shared<'shield, V, T1, T2, N1, N2>>
    where
        S: Shield<'collector>,
    {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        let result = self
            .data
            .compare_exchange_weak(current_raw, new_raw, success, failure);

        map_both(result, |raw| unsafe { Shared::from_raw(raw) })
    }
}

unsafe impl<'shield, V, T1, T2, const N1: usize, const N2: usize> Send for Atomic<V, T1, T2, N1, N2>
where
    T1: Tag<N1>,
    T2: Tag<N2>,
{
}

unsafe impl<'shield, V, T1, T2, const N1: usize, const N2: usize> Sync for Atomic<V, T1, T2, N1, N2>
where
    T1: Tag<N1>,
    T2: Tag<N2>,
{
}

impl<'shield, V, T1, T2, const N1: usize, const N2: usize> Unpin for Atomic<V, T1, T2, N1, N2>
where
    T1: Tag<N1>,
    T2: Tag<N2>,
{
}

impl<'shield, V, T1, T2, const N1: usize, const N2: usize> fmt::Debug for Atomic<V, T1, T2, N1, N2>
where
    T1: Tag<N1>,
    T2: Tag<N2>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::tag;
        let data = self.data.load(Ordering::SeqCst);
        let lo = tag::read_tag::<T1, N1>(data, tag::TagPosition::Lo);
        let hi = tag::read_tag::<T2, N2>(data, tag::TagPosition::Hi);

        f.debug_struct("Atomic")
            .field("raw", &data)
            .field("low_tag", &lo)
            .field("high_tag", &hi)
            .finish()
    }
}
