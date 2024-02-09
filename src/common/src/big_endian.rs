use core::fmt::{Debug, Display};

use crate::numbers::Number;

#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct BigEndian<T: Number>(pub T);

impl<T: Number> BigEndian<T> {
    pub fn get(&self) -> T {
        T::from_be(self.0)
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
