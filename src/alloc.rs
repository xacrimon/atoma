// if you thought the `barrier` and `deferred` modules were cursed, hoo boy are you in for a surprise
// this is probably UB as shit but it gets the job done, feel free to pr if you have a better solution, i don't

use core::{
    fmt,
    mem::{self, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr,
};

#[cfg(feature = "std")]
use std::alloc as stdalloc;

#[cfg(feature = "std")]
pub struct GlobalAllocator;

#[cfg(feature = "std")]
unsafe impl VirtualAllocRef for GlobalAllocator {
    fn alloc(&self, layout: &Layout) -> *mut u8 {
        unsafe {
            let std_layout =
                stdalloc::Layout::from_size_align_unchecked(layout.size(), layout.align());

            stdalloc::alloc(std_layout)
        }
    }

    unsafe fn dealloc(&self, layout: &Layout, ptr: *mut u8) {
        let std_layout = stdalloc::Layout::from_size_align_unchecked(layout.size(), layout.align());
        stdalloc::dealloc(ptr, std_layout)
    }

    fn clone_to_untyped(&self) -> AllocRef {
        AllocRef::new(Self)
    }
}

const INLINE_DYN_SPACE: usize = 24;

pub struct AllocRef {
    data: MaybeUninit<[u8; INLINE_DYN_SPACE]>,
    vtable: usize,
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
        let fat_ptr = &backing as &dyn VirtualAllocRef;
        let vtable = unsafe { mem::transmute::<&dyn VirtualAllocRef, [usize; 2]>(fat_ptr)[1] };

        unsafe {
            ptr::write(ptr, backing);
        }

        Self { data, vtable }
    }
}

impl Deref for AllocRef {
    type Target = dyn VirtualAllocRef;

    fn deref(&self) -> &Self::Target {
        let object_ptr = self.data.as_ptr() as usize;
        unsafe { mem::transmute::<[usize; 2], &Self::Target>([object_ptr, self.vtable]) }
    }
}

impl DerefMut for AllocRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let object_ptr = self.data.as_mut_ptr() as usize;
        unsafe { mem::transmute::<[usize; 2], &mut Self::Target>([object_ptr, self.vtable]) }
    }
}

impl Clone for AllocRef {
    fn clone(&self) -> Self {
        self.clone_to_untyped()
    }
}

impl Drop for AllocRef {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.deref_mut());
        }
    }
}

pub unsafe trait VirtualAllocRef: Send + Sync + 'static {
    fn alloc(&self, layout: &Layout) -> *mut u8;
    unsafe fn dealloc(&self, layout: &Layout, ptr: *mut u8);
    fn clone_to_untyped(&self) -> AllocRef;
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
