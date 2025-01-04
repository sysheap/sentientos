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

pub struct FatPointer<Ptr> {
    ptr: Ptr,
    len: usize,
}

impl<Ptr: Clone + Copy> FatPointer<Ptr> {
    fn new(ptr: Ptr, len: usize) -> Self {
        Self { ptr, len }
    }

    pub fn ptr(&self) -> Ptr {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

pub trait AsFatPointer {
    type T;
    fn as_fat_pointer(&self) -> FatPointer<*const Self::T>;
}

pub trait AsFatPointerMut {
    type T;
    fn as_fat_pointer_mut(&mut self) -> FatPointer<*mut Self::T>;
}

impl AsFatPointer for &str {
    type T = u8;

    fn as_fat_pointer(&self) -> FatPointer<*const u8> {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T> AsFatPointer for &[T] {
    type T = T;

    fn as_fat_pointer(&self) -> FatPointer<*const T> {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T> AsFatPointerMut for &mut [T] {
    type T = T;

    fn as_fat_pointer_mut(&mut self) -> FatPointer<*mut T> {
        FatPointer::new(self.as_mut_ptr(), self.len())
    }
}
