use crate::tag::{read_tag, set_tag, strip, NullTag, Tag, TagPosition};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ptr;

/// A `Shared` represents a tagged pointer.
/// It provides various utility methods for type conversion
/// and tag manipulation. In addition it is the only pointer type
/// that can be used to interact with `Atomic` since this type
/// enforces a lifetime based on the shield used to create it.
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
        unsafe { Self::from_raw(ptr::null::<()>() as usize) }
    }

    /// Constructs a `Shared` from a raw tagged pointer with an arbitrary lifetime.
    ///
    /// # Safety
    /// The alignment of `V` must free up sufficient low bits so that `T` fits.
    pub unsafe fn from_ptr(ptr: *mut V) -> Self {
        Self::from_raw(ptr as usize)
    }

    /// Constructs a `Shared` from a raw tagged pointer represented as an integer with an arbitrary lifetime.
    ///
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

    /// Get the raw tagged pointer as an integer.
    pub fn into_raw(self) -> usize {
        self.data
    }

    /// Get the raw tagged pointer.
    pub fn as_ptr(self) -> *mut V {
        self.data as *mut V
    }

    /// Remove all tags by zeroing their bits.
    pub fn strip(self) -> Self {
        let data = strip::<T1, T2>(self.data);
        unsafe { Self::from_raw(data) }
    }

    /// Converts the pointer into a reference.
    /// This will panic if the tagged pointer is null.
    ///
    /// # Safety
    /// - The pointer must either be null or point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed mutably.
    pub unsafe fn as_ref(self) -> Option<&'shield V> {
        self.as_ptr().as_ref()
    }

    /// Converts the pointer into a mutable reference.
    /// This will panic if the tagged pointer is null.
    ///
    /// # Safety
    /// - The pointer must either be null or point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed.
    pub unsafe fn as_mut_ref(self) -> Option<&'shield mut V> {
        let ptr = self.as_ptr();

        if !ptr.is_null() {
            Some(&mut *ptr)
        } else {
            None
        }
    }

    /// Converts the pointer into a reference.
    ///
    /// # Safety
    /// - The pointer must point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed mutably.
    pub unsafe fn as_ref_unchecked(self) -> &'shield V {
        &*self.as_ptr()
    }

    /// Converts the pointer into a mutable reference.
    ///
    /// # Safety
    /// - The pointer must point to a valid instance of `V`.
    /// - You must ensure the instance of `V` is not borrowed.
    pub unsafe fn as_mut_ref_unchecked(self) -> &'shield mut V {
        &mut *self.as_ptr()
    }

    /// Check if the tagged pointer is null.
    pub fn is_null(self) -> bool {
        self.as_ptr().is_null()
    }

    /// Get the tag in the low position.
    pub fn tag_lo(self) -> T1 {
        let bits = read_tag::<T1>(self.data, TagPosition::Lo);
        Tag::deserialize(bits)
    }

    /// Get the tag in the high position.
    pub fn tag_hi(self) -> T2 {
        let bits = read_tag::<T2>(self.data, TagPosition::Hi);
        Tag::deserialize(bits)
    }

    /// Set the tag in the low position.
    pub fn with_tag_lo(self, tag: T1) -> Self {
        let bits = tag.serialize();
        let data = set_tag::<T1>(self.data, bits, TagPosition::Lo);
        unsafe { Self::from_raw(data) }
    }

    /// Set the tag in the high position.
    pub fn with_tag_hi(self, tag: T2) -> Self {
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
