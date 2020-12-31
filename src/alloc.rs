// if you thought the `barrier` and `deferred` modules were cursed, hoo boy are you in for a surprise
// this is probably UB as shit but it gets the job done, feel free to pr if you have a better solution, i don't

use core::{
    fmt,
    mem::{self, MaybeUninit},
    ptr,
};

#[cfg(feature = "std")]
use std::alloc as stdalloc;

#[cfg(feature = "std")]
pub struct GlobalAllocator;

#[cfg(feature = "std")]
unsafe impl VirtualAllocRef for GlobalAllocator {
    unsafe fn alloc(&self, layout: &Layout) -> *mut u8 {
        let std_layout = stdalloc::Layout::from_size_align_unchecked(layout.size(), layout.align());
        stdalloc::alloc(std_layout)
    }

    unsafe fn dealloc(&self, layout: &Layout, ptr: *mut u8) {
        let std_layout = stdalloc::Layout::from_size_align_unchecked(layout.size(), layout.align());
        stdalloc::dealloc(ptr, std_layout)
    }

    fn clone_untyped(&self) -> AllocRef {
        AllocRef::new(Self)
    }
}

pub struct AllocatorMeta {
    alloc: fn(*const MaybeUninit<[u8; INLINE_DYN_SPACE]>, &Layout) -> *mut u8,
    dealloc: fn(*const MaybeUninit<[u8; INLINE_DYN_SPACE]>, &Layout, *mut u8),
    clone: fn(*const MaybeUninit<[u8; INLINE_DYN_SPACE]>) -> AllocRef,
    drop: fn(*const MaybeUninit<[u8; INLINE_DYN_SPACE]>),
}

const INLINE_DYN_SPACE: usize = 24;

pub struct AllocRef {
    data: MaybeUninit<[u8; INLINE_DYN_SPACE]>,
    meta: fn() -> AllocatorMeta,
}

impl AllocRef {
    pub fn new<T>(backing: T) -> Self
    where
        T: VirtualAllocRef,
    {
        assert!(
            mem::size_of::<T>() <= INLINE_DYN_SPACE && mem::align_of::<T>() <= INLINE_DYN_SPACE
        );

        let mut data = MaybeUninit::uninit();
        let ptr = data.as_mut_ptr() as *mut T;
        let meta = T::meta;

        unsafe {
            ptr::write(ptr, backing);
        }

        Self { data, meta }
    }

    pub fn alloc(&self, layout: &Layout) -> *mut u8 {
        (self.meta().alloc)(&self.data, layout)
    }

    pub fn dealloc(&self, layout: &Layout, ptr: *mut u8) {
        (self.meta().dealloc)(&self.data, layout, ptr);
    }

    fn meta(&self) -> AllocatorMeta {
        (self.meta)()
    }
}

impl Clone for AllocRef {
    fn clone(&self) -> Self {
        (self.meta().clone)(&self.data)
    }
}

impl Drop for AllocRef {
    fn drop(&mut self) {
        (self.meta().drop)(&self.data);
    }
}

pub unsafe trait VirtualAllocRef: Send + Sync + 'static {
    unsafe fn alloc(&self, layout: &Layout) -> *mut u8;
    unsafe fn dealloc(&self, layout: &Layout, ptr: *mut u8);
    fn clone_untyped(&self) -> AllocRef;

    #[doc(hidden)]
    unsafe fn drop_in_place(&mut self) {
        ptr::drop_in_place(self);
    }

    #[doc(hidden)]
    fn meta() -> AllocatorMeta {
        unsafe {
            AllocatorMeta {
                alloc: mem::transmute(Self::alloc as usize),
                dealloc: mem::transmute(Self::dealloc as usize),
                clone: mem::transmute(Self::clone_untyped as usize),
                drop: mem::transmute(Self::drop_in_place as usize),
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    size_: usize,
    align_: usize,
}

impl Layout {
    pub fn new<T>() -> Self {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        unsafe { Self::from_size_align_unchecked(size, align) }
    }

    pub unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Self {
        Self {
            size_: size,
            align_: align,
        }
    }

    pub fn size(&self) -> usize {
        self.size_
    }

    pub fn align(&self) -> usize {
        self.align_
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LayoutErr {
    private: (),
}

impl fmt::Display for LayoutErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid parameters to Layout::from_size_align")
    }
}
