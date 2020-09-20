use generic_array::{
    typenum::{Unsigned, U0},
    ArrayLength, GenericArray,
};
use std::mem;

pub enum TagPosition {
    Lo,
    Hi,
}

impl TagPosition {
    fn to_skip<T: Tag>(&self) -> usize {
        match self {
            TagPosition::Lo => 0,
            TagPosition::Hi => {
                let usize_bits = mem::size_of::<usize>() * 8;
                usize_bits - <T::Size as Unsigned>::to_usize()
            }
        }
    }
}

pub fn strip<T1: Tag, T2: Tag>(data: usize) -> usize {
    let mask1: usize = std::usize::MAX >> <T1::Size as Unsigned>::to_usize();
    let mask2: usize = std::usize::MAX << <T2::Size as Unsigned>::to_usize();
    data & mask1 & mask2
}

pub fn read_tag<T: Tag>(data: usize, position: TagPosition) -> GenericArray<bool, T::Size> {
    let to_skip = position.to_skip::<T>();
    let mut array = GenericArray::default();

    array
        .iter_mut()
        .enumerate()
        .skip(to_skip)
        .for_each(|(index, bit)| *bit = ((data >> index) & 1) == 1);

    array
}

pub fn set_tag<T: Tag>(
    mut data: usize,
    bits: GenericArray<bool, T::Size>,
    position: TagPosition,
) -> usize {
    let to_skip = position.to_skip::<T>();

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
pub trait Tag {
    type Size: ArrayLength<bool>;

    fn deserialize(bits: GenericArray<bool, Self::Size>) -> Self;
    fn serialize(self) -> GenericArray<bool, Self::Size>;
}

#[derive(Debug, Clone, Copy)]
pub struct NullTag;

impl Tag for NullTag {
    type Size = U0;

    #[inline]
    fn deserialize(_bits: GenericArray<bool, Self::Size>) -> Self {
        Self
    }

    #[inline]
    fn serialize(self) -> GenericArray<bool, Self::Size> {
        GenericArray::default()
    }
}
