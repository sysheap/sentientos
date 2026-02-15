use core::{
    fmt::Display,
    ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Rem, Shl, Shr, Sub},
};

use crate::memory::PAGE_SIZE;

pub fn align_up_page_size(value: usize) -> usize {
    align_up(value, PAGE_SIZE)
}

pub const fn align_up(value: usize, alignment: usize) -> usize {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

pub fn align_down_ptr<T>(ptr: *const T, alignment: usize) -> *const T {
    assert!(
        alignment.is_power_of_two(),
        "alignment must be a power of two"
    );
    ptr.mask(!(alignment - 1))
}

#[cfg(miri)]
pub fn align_down(value: usize, alignment: usize) -> usize {
    assert!(
        alignment.is_power_of_two(),
        "alignment must be a power of two"
    );
    value & !(alignment - 1)
}

pub struct PrintMemorySizeHumanFriendly(pub usize);

impl Display for PrintMemorySizeHumanFriendly {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut size = self.0 as f64;
        for format in ["", "KiB", "MiB", "GiB"] {
            if size < 1024.0 {
                return write!(f, "{size:.2} {format}");
            }
            size /= 1024.0;
        }
        write!(f, "{size:.2} TiB")
    }
}

pub fn copy_slice<T: Copy>(src: &[T], dst: &mut [T]) {
    assert!(dst.len() >= src.len());
    dst[..src.len()].copy_from_slice(src);
}

pub const fn minimum_amount_of_pages(value: usize) -> usize {
    align_up(value, PAGE_SIZE) / PAGE_SIZE
}

pub trait BufferExtension {
    fn interpret_as<T>(&self) -> &T;
    fn split_as<T>(&self) -> (&T, &[u8]);
}

impl BufferExtension for [u8] {
    fn interpret_as<T>(&self) -> &T {
        // SAFETY: Size and alignment are verified by assertions. The lifetime
        // of the returned reference is tied to &self.
        unsafe {
            assert!(self.len() >= core::mem::size_of::<T>());
            let ptr: *const T = self.as_ptr() as *const T;
            assert!(
                ptr.is_aligned(),
                "pointer not aligned for {}",
                core::any::type_name::<T>()
            );
            &*ptr
        }
    }

    fn split_as<T>(&self) -> (&T, &[u8]) {
        let (header_bytes, rest) = self.split_at(core::mem::size_of::<T>());
        (header_bytes.interpret_as(), rest)
    }
}

pub trait ByteInterpretable {
    fn as_slice(&self) -> &[u8] {
        // SAFETY: It is always safe to interpret a allocated struct as bytes
        unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of_val(self))
        }
    }
}

pub fn is_power_of_2_or_zero<DataType>(value: DataType) -> bool
where
    DataType:
        BitAnd<Output = DataType> + PartialEq<DataType> + From<u8> + Sub<Output = DataType> + Copy,
{
    value & (value - DataType::from(1)) == DataType::from(0)
}

pub fn is_aligned<DataType>(value: DataType, alignment: DataType) -> bool
where
    DataType: Rem<DataType, Output = DataType> + PartialEq<DataType> + From<u8>,
{
    value % alignment == DataType::from(0)
}

pub fn set_or_clear_bit<DataType>(
    data: &mut DataType,
    should_set_bit: bool,
    bit_position: usize,
) -> DataType
where
    DataType: BitOrAssign
        + BitAndAssign
        + Not<Output = DataType>
        + From<u8>
        + Shl<usize, Output = DataType>
        + Copy,
{
    if should_set_bit {
        set_bit(data, bit_position);
    } else {
        clear_bit(data, bit_position)
    }
    *data
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
) -> DataType
where
    DataType: BitAndAssign
        + BitOrAssign
        + Not<Output = DataType>
        + From<u8>
        + Shl<usize, Output = DataType>
        + Copy,
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
    *data
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

pub trait InBytes {
    fn in_bytes(&self) -> usize;
}

impl<T> InBytes for alloc::vec::Vec<T> {
    fn in_bytes(&self) -> usize {
        self.len() * core::mem::size_of::<T>()
    }
}

impl<T, const N: usize> InBytes for [T; N] {
    fn in_bytes(&self) -> usize {
        N * core::mem::size_of::<T>()
    }
}

#[cfg(test)]
mod tests {
    use crate::memory::PAGE_SIZE;

    #[test_case]
    fn align_up() {
        assert_eq!(super::align_up(26, 4), 28);
        assert_eq!(super::align_up(37, 3), 39);
        assert_eq!(super::align_up(64, 2), 64);
    }

    #[test_case]
    fn align_up_number_of_pages() {
        assert_eq!(super::minimum_amount_of_pages(PAGE_SIZE - 15), 1);
        assert_eq!(super::minimum_amount_of_pages(PAGE_SIZE + 15), 2);
        assert_eq!(super::minimum_amount_of_pages(PAGE_SIZE * 2), 2);
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
        assert!(super::get_bit(value, 0));
        assert!(!super::get_bit(value, 1));
        assert!(super::get_bit(value, 2));
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

    #[test_case]
    fn split_as_parses_header_and_remainder() {
        use super::BufferExtension;

        #[repr(C)]
        struct Header {
            tag: u16,
            len: u16,
            flags: u8,
        }

        let payload = [0xAA, 0xBB, 0xCC];
        let total_len = core::mem::size_of::<Header>() + payload.len();
        // Allocate with Header alignment so interpret_as's is_aligned() check is guaranteed.
        let layout =
            alloc::alloc::Layout::from_size_align(total_len, core::mem::align_of::<Header>())
                .expect("Layout must be valid");
        let buf = unsafe {
            let ptr = alloc::alloc::alloc_zeroed(layout);
            core::slice::from_raw_parts_mut(ptr, total_len)
        };
        buf[0..2].copy_from_slice(&0xCAFEu16.to_ne_bytes());
        buf[2..4].copy_from_slice(&128u16.to_ne_bytes());
        buf[4] = 0x07;
        buf[core::mem::size_of::<Header>()..].copy_from_slice(&payload);

        let (header, rest) = buf.split_as::<Header>();

        assert_eq!(header.tag, 0xCAFE);
        assert_eq!(header.len, 128);
        assert_eq!(header.flags, 0x07);
        assert_eq!(rest, &payload);

        unsafe { alloc::alloc::dealloc(buf.as_mut_ptr(), layout) };
    }
}
