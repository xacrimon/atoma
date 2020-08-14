use generic_array::{typenum::Unsigned, ArrayLength, GenericArray};

pub fn strip<T: Tag>(data: usize) -> usize {
    let mask: usize = usize::MAX >> <T::Size as Unsigned>::to_usize();
    data & mask
}

pub fn read_tag<T: Tag>(data: usize) -> GenericArray<bool, T::Size> {
    let mut array = GenericArray::default();

    array
        .iter_mut()
        .enumerate()
        .for_each(|(index, bit)| *bit = ((data >> index) & 1) == 1);

    array
}

pub fn set_tag<T: Tag>(mut data: usize, bits: GenericArray<bool, T::Size>) -> usize {
    bits.iter().enumerate().for_each(|(index, bit)| {
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
