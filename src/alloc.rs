use core::{fmt, mem};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    size_: usize,
    align_: usize,
}

impl Layout {
    pub fn new<T>() -> Self {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        unsafe { Self::from_size_align_unchecked(size, align)}
    }

    pub unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Self {
        Self { size_: size, align_: align }
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
