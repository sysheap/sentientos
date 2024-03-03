use core::{
    arch::asm,
    ops::{Deref, DerefMut},
};

#[allow(clippy::upper_case_acronyms)]
pub struct MMIO<T: Sized> {
    address: *mut T,
}

impl<Size> MMIO<Size> {
    pub const unsafe fn new(address: usize) -> Self {
        Self {
            address: address as *mut Size,
        }
    }

    pub unsafe fn add(&self, count: usize) -> Self {
        unsafe {
            Self {
                address: self.address.add(count),
            }
        }
    }
}

impl<T> Deref for MMIO<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // The Rust default is memory globber
            // Use it to force re-read of assembly
            asm!("");
            &*self.address
        }
    }
}

impl<T> DerefMut for MMIO<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            // The Rust default is memory globber
            // Use it to force re-read of assembly
            asm!("");
            &mut *self.address
        }
    }
}
