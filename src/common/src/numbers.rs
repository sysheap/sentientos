use super::consumable_buffer::FromU8Buffer;
use core::fmt::{Debug, Display};

pub trait Number: Debug + Display + Copy + Clone {
    fn from_be(value: Self) -> Self;
    fn from_le_bytes(bytes: &[u8]) -> Self;
}

impl<T: Number> FromU8Buffer for T {
    fn from_u8_buffer(buffer: &[u8]) -> Self {
        T::from_le_bytes(buffer)
    }
}

macro_rules! impl_number {
    ($T:ty) => {
        impl Number for $T {
            fn from_be(value: Self) -> Self {
                <$T>::from_be(value)
            }
            fn from_le_bytes(bytes: &[u8]) -> Self {
                <$T>::from_le_bytes(bytes.try_into().unwrap())
            }
        }
    };
}

impl_number!(u8); // Not really needed but we keep using BigEndian<u8> in network structs for understandability
impl_number!(u16);
impl_number!(u32);
impl_number!(u64);
impl_number!(u128);
