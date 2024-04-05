use core::fmt::{Debug, Display};

use crate::{consumable_buffer::FromU8Buffer, numbers::Number};

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct BigEndian<T: Number>(T);

impl<T: Number> BigEndian<T> {
    pub fn from_little_endian(value: T) -> Self {
        // Use from_be to invert byte order
        BigEndian(T::from_be(value))
    }

    pub fn get(&self) -> T {
        T::from_be(self.0)
    }
}

impl<T: Number> FromU8Buffer for BigEndian<T> {
    fn from_u8_buffer(buffer: &[u8]) -> Self {
        BigEndian(T::from_le_bytes(buffer))
    }
}

impl<T: Number> Debug for BigEndian<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl<T: Number> Display for BigEndian<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}
