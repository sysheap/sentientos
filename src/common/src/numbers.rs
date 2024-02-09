use core::fmt::{Debug, Display};

pub trait Number: Debug + Display + Copy + Clone {
    fn from_be(value: Self) -> Self;
}

macro_rules! impl_number {
    ($T:ty) => {
        impl Number for $T {
            fn from_be(value: Self) -> Self {
                <$T>::from_be(value)
            }
        }
    };
}

impl_number!(u32);
