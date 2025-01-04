use crate::pointer::{AsFatPointer, FatPointer};

auto trait IsValue {}

impl<T> !IsValue for &T {}
impl<T> !IsValue for &mut T {}

/// This trait keeps T as T if T is not a reference
/// &T is converted to *const T
/// &mut T is converted to *mut T
pub trait RefToPointer<T> {
    type Out;
    fn to_pointer_if_ref(self) -> Self::Out;
    #[allow(clippy::missing_safety_doc)]
    unsafe fn to_ref_if_pointer(input: Self::Out) -> Self;
}

impl<T: IsValue> RefToPointer<T> for T {
    type Out = T;

    fn to_pointer_if_ref(self) -> Self::Out {
        self
    }

    unsafe fn to_ref_if_pointer(input: Self::Out) -> Self {
        input
    }
}

impl<T> RefToPointer<T> for &T {
    type Out = *const T;

    fn to_pointer_if_ref(self) -> Self::Out {
        self
    }

    unsafe fn to_ref_if_pointer(input: Self::Out) -> Self {
        &*input
    }
}

impl<T> RefToPointer<T> for &mut T {
    type Out = *mut T;

    fn to_pointer_if_ref(self) -> Self::Out {
        self
    }

    unsafe fn to_ref_if_pointer(input: Self::Out) -> Self {
        &mut *input
    }
}

impl RefToPointer<&str> for &str {
    type Out = FatPointer<*const u8>;

    fn to_pointer_if_ref(self) -> Self::Out {
        self.to_fat_pointer()
    }

    unsafe fn to_ref_if_pointer(input: Self::Out) -> Self {
        core::str::from_raw_parts(input.ptr(), input.len())
    }
}

impl RefToPointer<&[u8]> for &[u8] {
    type Out = FatPointer<*const u8>;

    fn to_pointer_if_ref(self) -> Self::Out {
        self.to_fat_pointer()
    }

    unsafe fn to_ref_if_pointer(input: Self::Out) -> Self {
        core::slice::from_raw_parts(input.ptr(), input.len())
    }
}

impl RefToPointer<&mut [u8]> for &mut [u8] {
    type Out = FatPointer<*mut u8>;

    fn to_pointer_if_ref(self) -> Self::Out {
        self.to_fat_pointer()
    }

    unsafe fn to_ref_if_pointer(input: Self::Out) -> Self {
        core::slice::from_raw_parts_mut(input.ptr(), input.len())
    }
}
