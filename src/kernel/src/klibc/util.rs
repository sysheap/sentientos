use core::ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl, Shr};

use crate::memory::PAGE_SIZE;

pub fn align_up(value: usize, alignment: usize) -> usize {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

pub fn align_up_and_get_number_of_pages(value: usize) -> usize {
    align_up(value, PAGE_SIZE) / PAGE_SIZE
}

pub fn copy_slice<T: Copy>(src: &[T], dst: &mut [T]) {
    assert!(dst.len() >= src.len());
    dst[..src.len()].copy_from_slice(src);
}

pub fn set_or_clear_bit<DataType>(data: &mut DataType, should_set_bit: bool, bit_position: usize)
where
    DataType: BitOrAssign
        + BitAndAssign
        + Not<Output = DataType>
        + From<u8>
        + Shl<usize, Output = DataType>,
{
    if should_set_bit {
        set_bit(data, bit_position);
    } else {
        clear_bit(data, bit_position)
    }
}

pub fn set_bit<DataType>(data: &mut DataType, bit_position: usize)
where
    DataType: BitOrAssign + Not<Output = DataType> + From<u8> + Shl<usize, Output = DataType>,
{
    *data |= DataType::from(1) << bit_position
}

pub fn clear_bit<DataType>(data: &mut DataType, bit_position: usize)
where
    DataType: BitAndAssign + Not<Output = DataType> + From<u8> + Shl<usize, Output = DataType>,
{
    *data &= !(DataType::from(1) << bit_position)
}

pub fn get_bit<DataType>(data: DataType, bit_position: usize) -> bool
where
    DataType: Shr<usize, Output = DataType>
        + BitAnd<DataType, Output = DataType>
        + PartialEq<DataType>
        + From<u8>,
{
    ((data >> bit_position) & DataType::from(0x1)) == DataType::from(1)
}

pub fn set_multiple_bits<DataType, ValueType>(
    data: &mut DataType,
    value: ValueType,
    number_of_bits: usize,
    bit_position: usize,
) where
    DataType: BitAndAssign
        + BitOrAssign
        + Not<Output = DataType>
        + From<u8>
        + Shl<usize, Output = DataType>,
    ValueType: Copy + BitAnd + From<u8> + Shl<usize, Output = ValueType>,
    <ValueType as BitAnd>::Output: PartialOrd<ValueType>,
{
    let mut mask: DataType = !(DataType::from(0));

    for idx in 0..number_of_bits {
        mask &= !(DataType::from(1) << (bit_position + idx));
    }

    *data &= mask;

    mask = DataType::from(0);

    for idx in 0..number_of_bits {
        if (value & (ValueType::from(1) << idx)) > ValueType::from(0) {
            mask |= DataType::from(1) << (bit_position + idx);
        }
    }

    *data |= mask;
}

pub fn get_multiple_bits<DataType, ValueType>(
    data: DataType,
    number_of_bits: usize,
    bit_position: usize,
) -> ValueType
where
    DataType: Shr<usize, Output = DataType> + BitAnd<u64, Output = ValueType>,
{
    (data >> bit_position) & (2u64.pow(number_of_bits as u32) - 1)
}

#[cfg(test)]
mod tests {
    use crate::memory::page_allocator::PAGE_SIZE;

    #[test_case]
    fn align_up() {
        assert_eq!(super::align_up(26, 4), 28);
        assert_eq!(super::align_up(37, 3), 39);
        assert_eq!(super::align_up(64, 2), 64);
    }

    #[test_case]
    fn align_up_number_of_pages() {
        assert_eq!(super::align_up_and_get_number_of_pages(PAGE_SIZE - 15), 1);
        assert_eq!(super::align_up_and_get_number_of_pages(PAGE_SIZE + 15), 2);
        assert_eq!(super::align_up_and_get_number_of_pages(PAGE_SIZE * 2), 2);
    }

    #[test_case]
    fn copy_from_slice() {
        let src = [1, 2, 3, 4, 5];
        let mut dst = [0, 0, 0, 0, 0, 0, 0];
        super::copy_slice(&src, &mut dst);
        assert_eq!(dst, [1, 2, 3, 4, 5, 0, 0]);
    }

    #[test_case]
    fn set_or_clear_bit() {
        let mut value: u64 = 0b1101101;
        super::set_or_clear_bit(&mut value, true, 1);
        assert_eq!(value, 0b1101111);
        super::set_or_clear_bit(&mut value, false, 1);
        assert_eq!(value, 0b1101101);
        super::set_or_clear_bit(&mut value, false, 0);
        assert_eq!(value, 0b1101100);
    }

    #[test_case]
    fn set_bit() {
        let mut value: u64 = 0b1101110;
        super::set_bit(&mut value, 0);
        assert_eq!(value, 0b1101111);
        super::set_bit(&mut value, 4);
        assert_eq!(value, 0b1111111);
    }

    #[test_case]
    fn clear_bit() {
        let mut value: u64 = 0b1101111;
        super::clear_bit(&mut value, 0);
        assert_eq!(value, 0b1101110);
        super::clear_bit(&mut value, 5);
        assert_eq!(value, 0b1001110);
        super::clear_bit(&mut value, 0);
        assert_eq!(value, 0b1001110);
    }

    #[test_case]
    fn get_bit() {
        let value: u64 = 0b1101101;
        assert_eq!(super::get_bit(value, 0), true);
        assert_eq!(super::get_bit(value, 1), false);
        assert_eq!(super::get_bit(value, 2), true);
    }

    #[test_case]
    fn set_multiple_bits() {
        let mut value: u64 = 0b1101101;
        super::set_multiple_bits(&mut value, 0b111, 3, 0);
        assert_eq!(value, 0b1101111);
        super::set_multiple_bits(&mut value, 0b110, 3, 1);
        assert_eq!(value, 0b1101101);
        super::set_multiple_bits(&mut value, 0b011, 3, 2);
        assert_eq!(value, 0b1101101);
    }

    #[test_case]
    fn get_multiple_bits() {
        let value: u64 = 0b1101101;
        assert_eq!(super::get_multiple_bits(value, 3, 0), 0b101);
        assert_eq!(super::get_multiple_bits(value, 3, 1), 0b110);
        assert_eq!(super::get_multiple_bits(value, 3, 2), 0b011);
    }
}
