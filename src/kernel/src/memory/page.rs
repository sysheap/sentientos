use core::{
    ops::{Deref, DerefMut, Range},
    ptr::NonNull,
};

pub const PAGE_SIZE: usize = 4096;

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

pub trait Pages {
    fn zero(&mut self) {
        let pages = self.as_slice();
        for page in pages {
            page.0.fill(0);
        }
    }

    fn as_slice(&mut self) -> &mut [Page];
}

impl Pages for Range<NonNull<Page>> {
    fn as_slice(&mut self) -> &mut [Page] {
        unsafe {
            let offset = self.end.offset_from(self.start);
            assert!(offset >= 0);
            core::slice::from_raw_parts_mut(self.start.as_ptr(), offset as usize)
        }
    }
}
