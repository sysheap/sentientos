/// This trait both abstracts *const T and *mut T
/// It can be used if a method can receive both types of pointers
pub trait Pointer: Clone + Copy {
    type Pointee;

    fn as_raw(&self) -> usize;
    fn as_pointer(ptr: usize) -> Self;
}

impl<T> Pointer for *const T {
    type Pointee = T;

    fn as_raw(&self) -> usize {
        *self as usize
    }

    fn as_pointer(ptr: usize) -> Self {
        ptr as *const T
    }
}

impl<T> Pointer for *mut T {
    type Pointee = T;

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

impl<Ptr: Pointer> FatPointer<Ptr> {
    pub fn new(ptr: Ptr, len: usize) -> Self {
        Self { ptr, len }
    }

    pub fn ptr(&self) -> Ptr {
        self.ptr
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }
}

pub trait AsFatPointer {
    type Ptr;
    fn to_fat_pointer(self) -> FatPointer<Self::Ptr>;
}

impl AsFatPointer for &str {
    type Ptr = *const u8;

    fn to_fat_pointer(self) -> FatPointer<Self::Ptr> {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T> AsFatPointer for &[T] {
    type Ptr = *const T;

    fn to_fat_pointer(self) -> FatPointer<Self::Ptr> {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T> AsFatPointer for &mut [T] {
    type Ptr = *mut T;

    fn to_fat_pointer(self) -> FatPointer<Self::Ptr> {
        FatPointer::new(self.as_mut_ptr(), self.len())
    }
}
