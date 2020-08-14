use crate::tag::{read_tag, set_tag, strip, Tag};
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct Shared<'shield, V: 'shield, T: Tag> {
    pub(crate) data: usize,
    _m0: PhantomData<&'shield ()>,
    _m1: PhantomData<V>,
    _m2: PhantomData<T>,
}

impl<'shield, V: 'shield, T: Tag> Shared<'shield, V, T> {
    pub fn null() -> Self {
        Self::from_data(0)
    }

    pub unsafe fn from_ptr(ptr: *mut V) -> Self {
        Self::from_data(ptr as usize)
    }

    pub fn from_data(data: usize) -> Self {
        Self {
            data,
            _m0: PhantomData,
            _m1: PhantomData,
            _m2: PhantomData,
        }
    }

    pub fn as_ptr(&self) -> *mut V {
        strip::<T>(self.data) as *mut V
    }

    pub unsafe fn as_ref(&self) -> Option<&'shield V> {
        self.as_ptr().as_ref()
    }

    pub unsafe fn as_mut_ref(&mut self) -> Option<&'shield mut V> {
        let ptr = self.as_ptr();

        if !ptr.is_null() {
            Some(&mut *ptr)
        } else {
            None
        }
    }

    pub unsafe fn as_ref_unchecked(&self) -> &'shield V {
        &*self.as_ptr()
    }

    pub unsafe fn as_mut_ref_unchecked(&mut self) -> &'shield mut V {
        &mut *self.as_ptr()
    }

    pub fn is_null(&self) -> bool {
        self.as_ptr().is_null()
    }

    pub fn tag(&self) -> T {
        let bits = read_tag::<T>(self.data);
        Tag::deserialize(bits)
    }

    pub fn with_tag(&self, tag: T) -> Self {
        let bits = tag.serialize();
        let data = set_tag::<T>(self.data, bits);
        Self::from_data(data)
    }
}
