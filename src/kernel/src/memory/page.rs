use core::{
    ops::{Deref, DerefMut, Range},
    ptr::NonNull,
};

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

#[cfg(test)]
mod tests {
    use core::{ops::Range, ptr::NonNull};

    use common::mutex::Mutex;

    use crate::memory::page::Pages;
    use crate::memory::page_allocator::MetadataPageAllocator;

    use super::{Page, PAGE_SIZE};

    static mut PAGE_ALLOC_MEMORY: [u8; PAGE_SIZE * 8] = [0; PAGE_SIZE * 8];
    static PAGE_ALLOC: Mutex<MetadataPageAllocator> = Mutex::new(MetadataPageAllocator::new());

    fn init_allocator() {
        unsafe {
            PAGE_ALLOC.lock().init(&mut PAGE_ALLOC_MEMORY);
        }
    }

    fn alloc(number_of_pages: usize) -> Option<Range<NonNull<Page>>> {
        PAGE_ALLOC.lock().alloc(number_of_pages)
    }

    fn dealloc(pages: Range<NonNull<Page>>) {
        PAGE_ALLOC.lock().dealloc(pages.start)
    }

    #[test_case]
    fn test_zero() {
        init_allocator();
        let mut pages = alloc(1).unwrap();
        let slice = pages.as_slice();
        slice[0].0.fill(0xff);
        pages.zero();
        assert_eq!(pages.as_slice(), &[Page([0; PAGE_SIZE])]);
        dealloc(pages);
    }

    #[test_case]
    fn test_slice_count() {
        init_allocator();
        let mut pages = alloc(2).unwrap();
        let slice = pages.as_slice();
        assert!(slice.len() == 2);
    }
}
