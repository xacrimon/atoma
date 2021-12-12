use core::ops::{Deref, DerefMut};

/// This struct has a minimum alignment that matches the cache prefetch size on different platforms.
/// This is often used to reduce false sharing in concurrent code by adding space between fields.
///
/// This type simplifies that task, just wrap a field in this and the compiler will take
/// care of aligning it properly.
#[cfg_attr(any(target_arch = "x86_64"), repr(align(128)))]
#[cfg_attr(
    not(any(target_arch = "x86_64")),
    repr(align(64))
)]
pub struct CachePadded<T> {
    value: T,
}

impl<T> CachePadded<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for CachePadded<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

#[cfg(test)]
mod tests {
    use super::CachePadded;
    use std::mem;

    #[test]
    fn check_size() {
        let alignment = if cfg!(target_arch = "x86_64") {
            128
        } else {
            64
        };

        assert_eq!(mem::size_of::<CachePadded<usize>>(), alignment);
    }

    #[test]
    fn check_align() {
        let alignment = if cfg!(target_arch = "x86_64") {
            128
        } else {
            64
        };

        assert_eq!(mem::align_of::<CachePadded<usize>>(), alignment);
    }
}
