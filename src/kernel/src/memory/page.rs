use core::ops::{Deref, DerefMut};

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, PartialEq, Eq)]
#[repr(C, align(4096))]
pub struct Page([u8; PAGE_SIZE]);

impl Deref for Page {
    type Target = [u8; PAGE_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Page {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
