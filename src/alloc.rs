fn round_up(num: usize, factor: usize) -> usize {
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
        assert!(round_up(size, align) >= size);
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

#[cfg(test)]
mod tests {
    use super::{round_up, Layout};

    #[test]
    fn check_round_up() {
        assert_eq!(round_up(5, 32), 32);
        assert_eq!(round_up(753, 512), 1024);
        assert_eq!(round_up(753, 256), 768);
    }

    #[test]
    #[should_panic]
    fn incorrect_align_test() {
        Layout::new(16, 18);
        Layout::new(16, 1953);
    }
}
