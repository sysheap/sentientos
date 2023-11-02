use core::marker::PhantomData;

#[allow(clippy::upper_case_acronyms)]
pub struct MMIO<Size: Sized> {
    address: *mut Size,
    phantom: PhantomData<Size>,
}

impl<Size> MMIO<Size> {
    pub const fn new(address: usize) -> Self {
        Self {
            address: address as *mut Size,
            phantom: PhantomData,
        }
    }

    pub unsafe fn read(&self) -> Size {
        self.address.read_volatile()
    }

    pub unsafe fn write(&self, value: Size) {
        self.address.write_volatile(value);
    }

    pub fn add(&self, count: usize) -> Self {
        unsafe {
            Self {
                address: self.address.add(count),
                phantom: PhantomData,
            }
        }
    }
}
