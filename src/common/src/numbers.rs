use core::fmt::{Debug, Display};

pub trait Number: Debug + Display + Copy + Clone {
    fn from_be(value: Self) -> Self;
    fn from_le_bytes(bytes: &[u8]) -> Self;
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

impl_number!(u32);
