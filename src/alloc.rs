fn round_up_fp2(num: usize, factor: usize) -> usize {
    num.wrapping_add(factor)
        .wrapping_sub(1)
        .wrapping_sub((num.wrapping_add(factor).wrapping_sub(1)).wrapping_rem(factor))
}

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    size: usize,
    align: usize,
}

impl Layout {
    pub fn new(size: usize, align: usize) -> Self {
        assert!(align != 0 && align.is_power_of_two());
        assert!(round_up_fp2(size, align) >= size);
        unsafe { Self::from_size_align_unchecked(size, align) }
    }

    pub unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Self {
        Self { size, align }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn align(&self) -> usize {
        self.align
    }
}

pub trait Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout);
}

pub struct GlobalAllocator;

impl Allocator for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let std_layout =
            std::alloc::Layout::from_size_align_unchecked(layout.size(), layout.align());

        std::alloc::alloc(std_layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let std_layout =
            std::alloc::Layout::from_size_align_unchecked(layout.size(), layout.align());

        std::alloc::dealloc(ptr, std_layout)
    }
}

#[cfg(test)]
mod tests {
    use super::{round_up_fp2, Allocator, GlobalAllocator, Layout};

    #[test]
    fn check_round_up_fp2() {
        assert_eq!(round_up_fp2(5, 32), 32);
        assert_eq!(round_up_fp2(753, 512), 1024);
        assert_eq!(round_up_fp2(753, 256), 768);
    }

    #[test]
    #[should_panic]
    fn incorrect_align_test_1() {
        Layout::new(16, 18);
    }

    #[test]
    #[should_panic]
    fn incorrect_align_test_2() {
        Layout::new(16, 1953);
    }

    #[test]
    fn global_allocator() {
        let layout = Layout::new(1024, 64);

        unsafe {
            let a = GlobalAllocator.alloc(layout);
            let b = GlobalAllocator.alloc(layout);
            let c = GlobalAllocator.alloc(layout);
            let d = GlobalAllocator.alloc(layout);
            GlobalAllocator.dealloc(a, layout);
            GlobalAllocator.dealloc(b, layout);
            GlobalAllocator.dealloc(c, layout);
            GlobalAllocator.dealloc(d, layout);
        }
    }
}
