pub struct Userpointer<T> {
    ptr: *const T,
}

impl<T> Userpointer<T> {
    pub fn new(ptr: *const T) -> Self {
        Self { ptr }
    }

    pub fn get(&self) -> *const T {
        self.ptr
    }
}

pub struct UserpointerMut<T> {
    ptr: *mut T,
}

impl<T> UserpointerMut<T> {
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }
}
