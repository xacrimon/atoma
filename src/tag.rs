use core::mem;

pub enum TagPosition {
    Lo,
    Hi,
}

impl TagPosition {
    /// Calculates the start bit offset of the tag depending on the position and type.
    fn to_skip<T: Tag<N>, const N: usize>(&self) -> usize {
        match self {
            // low tags always start at 0
            TagPosition::Lo => 0,

            // high tags occupy the highest bits so the start offset is the max index minus the size
            TagPosition::Hi => {
                let usize_bits = mem::size_of::<usize>() * 8;
                usize_bits - N
            }
        }
    }
}

/// Zeroes all the tag bits.
pub fn strip<T1: Tag<N1>, T2: Tag<N2>, const N1: usize, const N2: usize>(data: usize) -> usize {
    // mask for zeroing the low tag
    let mask1: usize = core::usize::MAX >> N1;

    // mask for zeroing the high tag
    let mask2: usize = core::usize::MAX << N2;

    // apply the masks with an AND to zero the bits
    data & mask1 & mask2
}

/// Read the bits of a tag a a certain position.
pub fn read_tag<T: Tag<N>, const N: usize>(data: usize, position: TagPosition) -> [bool; N] {
    let to_skip = position.to_skip::<T, N>();
    let mut array = [false; N];

    array
        .iter_mut()
        .enumerate()
        .skip(to_skip)
        .for_each(|(index, bit)| *bit = ((data >> index) & 1) == 1);

    array
}

/// Set the bits of a tag at a certain position.
pub fn set_tag<T: Tag<N>, const N: usize>(
    mut data: usize,
    bits: [bool; N],
    position: TagPosition,
) -> usize {
    let to_skip = position.to_skip::<T, N>();

    bits.iter()
        .enumerate()
        .skip(to_skip)
        .for_each(|(index, bit)| {
            let value = if *bit { 1 } else { 0 };
            data = (data & !(1 << index)) | (value << index);
        });

    data
}

/// The `Tag` trait represents any struct that can be serialized
/// and packed into the unused bits of a pointer producing
/// a so called "tagged" pointer.
/// The amount of bits available are variable and the amount
/// you can use depends on whether the tag is in in the low or high position.
///
/// In low position you can use as many bits as must be zero due to
/// alignment. If you don't know the alignment of your pointer you can assume it is
/// that of the value it is pointing to. The amount of available bits in the low
/// position is the binary logarithm of the alignment in bytes.
///
/// In high position the number of available bits is determined by your compilation target.
/// On 32 bit architectures this number shall be assumed to be equal to 0.
/// On x86_64 with 4 level paging the number of available bits is 16 and with level
/// 5 paging it is 8 bits. On 64 bit ARM without pointer authentication you also have 16
/// available bits. With pointer authentication you can only reasonably assume you have 0 available
/// bits unless you know otherwise for your compiler. On all other architectures assume you have
/// 0 available bits unless you know otherwise.
pub trait Tag<const N: usize>: Copy {
    /// Deserialize an array of bits into the tag.
    fn deserialize(bits: [bool; N]) -> Self;

    /// Serialize the tag to an array of bits.
    fn serialize(self) -> [bool; N];
}

/// This tag is a placeholder type that has a size of 0 and stores no state.
/// If you don't have any tag with information you want to store, this is the default.
#[derive(Debug, Clone, Copy)]
pub struct NullTag;

impl Tag<0> for NullTag {
    fn deserialize(_bits: [bool; 0]) -> Self {
        Self
    }

    fn serialize(self) -> [bool; 0] {
        []
    }
}
