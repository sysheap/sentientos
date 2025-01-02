/// This trait both abstracts *const T and *mut T
/// It can be used if a method can receive both types of pointers
pub trait Pointer<T>: Clone + Copy {
    fn as_raw(&self) -> usize;
    fn as_pointer(ptr: usize) -> Self;
}

impl<T> Pointer<T> for *const T {
    fn as_raw(&self) -> usize {
        *self as usize
    }

    fn as_pointer(ptr: usize) -> Self {
        ptr as *const T
    }
}

impl<T> Pointer<T> for *mut T {
    fn as_raw(&self) -> usize {
        *self as usize
    }

    fn as_pointer(ptr: usize) -> Self {
        ptr as *mut T
    }
}
