use generic_array::{
    typenum::{UTerm, Unsigned},
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

pub trait Tag {
    type Size: ArrayLength<bool>;

    fn deserialize(bits: GenericArray<bool, Self::Size>) -> Self;
    fn serialize(self) -> GenericArray<bool, Self::Size>;
}

#[derive(Debug, Clone, Copy)]
pub struct NullTag;

impl Tag for NullTag {
    type Size = UTerm;

    fn deserialize(_bits: GenericArray<bool, Self::Size>) -> Self {
        Self
    }

    fn serialize(self) -> GenericArray<bool, Self::Size> {
        GenericArray::default()
    }
}
