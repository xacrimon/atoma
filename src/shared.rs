use crate::tag::{read_tag, set_tag, strip, NullTag, Tag, TagPosition};
use std::fmt::{self, Debug};
use std::marker::PhantomData;

#[repr(transparent)]
pub struct Shared<'shield, V, T1 = NullTag, T2 = NullTag>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
    pub(crate) data: usize,
    _m0: PhantomData<&'shield ()>,
    _m1: PhantomData<V>,
    _m2: PhantomData<T1>,
    _m3: PhantomData<T2>,
}

impl<'shield, V, T1, T2> Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
    pub fn null() -> Self {
        unsafe { Self::from_raw(0) }
    }

    /// # Safety
    /// The alignment of `V` must free up sufficient low bits so that `T` fits.
    pub unsafe fn from_ptr(ptr: *mut V) -> Self {
        Self::from_raw(ptr as usize)
    }

    /// This function constructs a `Shared<'shield, V, T>` from a raw tagged pointer.
    ///
    /// # Safety
    /// This is marked unsafe because extreme caution must be taken to
    /// supply correct data and ensure the lifetime is what you expect.
    pub unsafe fn from_raw(data: usize) -> Self {
        Self {
            data,
            _m0: PhantomData,
            _m1: PhantomData,
            _m2: PhantomData,
            _m3: PhantomData,
        }
    }

    pub fn into_raw(self) -> usize {
        self.data
    }

    pub fn as_ptr(&self) -> *mut V {
        self.data as *mut V
    }

    pub fn strip(&self) -> Self {
        let data = strip::<T1, T2>(self.data);
        unsafe { Self::from_raw(data) }
    }

    /// # Safety
    /// - The pointer must either be null or point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed mutably.
    pub unsafe fn as_ref(&self) -> Option<&'shield V> {
        self.as_ptr().as_ref()
    }

    /// # Safety
    /// - The pointer must either be null or point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed.
    pub unsafe fn as_mut_ref(&mut self) -> Option<&'shield mut V> {
        let ptr = self.as_ptr();

        if !ptr.is_null() {
            Some(&mut *ptr)
        } else {
            None
        }
    }

    /// # Safety
    /// - The pointer must point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed mutably.
    pub unsafe fn as_ref_unchecked(&self) -> &'shield V {
        &*self.as_ptr()
    }

    /// # Safety
    /// - The pointer must point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed.
    pub unsafe fn as_mut_ref_unchecked(&mut self) -> &'shield mut V {
        &mut *self.as_ptr()
    }

    pub fn is_null(&self) -> bool {
        self.as_ptr().is_null()
    }

    pub fn tag_lo(&self) -> T1 {
        let bits = read_tag::<T1>(self.data, TagPosition::Lo);
        Tag::deserialize(bits)
    }

    pub fn tag_hi(&self) -> T2 {
        let bits = read_tag::<T2>(self.data, TagPosition::Hi);
        Tag::deserialize(bits)
    }

    pub fn with_tag_lo(&self, tag: T1) -> Self {
        let bits = tag.serialize();
        let data = set_tag::<T1>(self.data, bits, TagPosition::Lo);
        unsafe { Self::from_raw(data) }
    }

    pub fn with_tag_hi(&self, tag: T2) -> Self {
        let bits = tag.serialize();
        let data = set_tag::<T2>(self.data, bits, TagPosition::Hi);
        unsafe { Self::from_raw(data) }
    }
}

impl<'shield, V, T1, T2> Clone for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
    fn clone(&self) -> Self {
        unsafe { Self::from_raw(self.data) }
    }
}

impl<'shield, V, T1, T2> Copy for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
}

impl<'shield, V, T1, T2> PartialEq for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
    fn eq(&self, other: &Self) -> bool {
        self.into_raw() == other.into_raw()
    }
}

impl<'shield, V, T1, T2> Eq for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
}

impl<'shield, V, T1, T2> Debug for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{:?}", self.data)
    }
}

unsafe impl<'shield, V, T1, T2> Send for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
}

unsafe impl<'shield, V, T1, T2> Sync for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
}

impl<'shield, V, T1, T2> Unpin for Shared<'shield, V, T1, T2>
where
    V: 'shield,
    T1: Tag,
    T2: Tag,
{
}
