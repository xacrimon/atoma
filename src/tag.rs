pub const MIN_ALIGN: usize = 4;
const STRIP_MASK: usize = usize::MAX >> 2;

pub fn strip(data: usize) -> usize {
    data & STRIP_MASK
}

pub fn read_tag(data: usize) -> [bool; 2] {
    fn read_bit(data: usize, index: usize) -> bool {
        ((data >> index) & 1) == 1
    }

    [read_bit(data, 0), read_bit(data, 1)]
}

pub fn set_tag(data: usize, bits: [bool; 2]) -> usize {
    fn set_bit(data: usize, index: usize, value: bool) -> usize {
        let value = if value { 1 } else { 0 };
        (data & !(1 << index)) | (value << index)
    }

    set_bit(data, 0, bits[0]) | set_bit(data, 1, bits[1])
}

pub trait Tag {
    fn from_bits(bits: [bool; 2]) -> Self;
    fn into_bits(self) -> [bool; 2];
}
