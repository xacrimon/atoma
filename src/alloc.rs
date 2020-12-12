use core::{
    fmt, mem,
    ops::{Deref, DerefMut},
    ptr,
};

const INLINE_DYN_SPACE: usize = 56;

pub struct AllocRef {
    data: [u8; INLINE_DYN_SPACE],
    vtable: usize,
}

impl AllocRef {
    pub fn new<T>(backing: T) -> Self
    where
        T: VirtualAllocRef,
    {
        let mut data = [0; INLINE_DYN_SPACE];
        let ptr = &mut data as *mut [u8; INLINE_DYN_SPACE] as *mut T;
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
        let object_ptr = &self.data as *const [u8; INLINE_DYN_SPACE] as usize;
        unsafe { mem::transmute::<[usize; 2], &Self::Target>([object_ptr, self.vtable]) }
    }
}

impl DerefMut for AllocRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let object_ptr = &mut self.data as *mut [u8; INLINE_DYN_SPACE] as usize;
        unsafe { mem::transmute::<[usize; 2], &mut Self::Target>([object_ptr, self.vtable]) }
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
    fn clone(&self) -> AllocRef;
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
