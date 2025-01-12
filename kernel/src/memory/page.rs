use alloc::boxed::Box;
use core::{
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::klibc::util::copy_slice;

pub const PAGE_SIZE: usize = 4096;

#[derive(PartialEq, Eq, Clone)]
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

impl core::fmt::Debug for Page {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Page({:p})", self.0.as_ptr())
    }
}

impl Page {
    pub(super) fn zero() -> Self {
        Self([0; PAGE_SIZE])
    }
}

pub trait Pages {
    fn as_u8_slice(&mut self) -> &mut [u8];
}

impl Pages for [Page] {
    fn as_u8_slice(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.as_mut_ptr() as *mut u8,
                core::mem::size_of_val(self),
            )
        }
    }
}

#[derive(Debug)]
pub struct PinnedHeapPages {
    allocation: Box<[Page]>,
}

impl PinnedHeapPages {
    pub fn new(number_of_pages: usize) -> Self {
        assert!(number_of_pages > 0);
        let allocation = vec![Page::zero(); number_of_pages].into_boxed_slice();
        Self { allocation }
    }

    pub fn fill(&mut self, data: &[u8]) {
        copy_slice(data, self.as_u8_slice());
    }

    pub fn as_mut_ptr(&mut self) -> NonNull<Page> {
        unsafe { NonNull::new_unchecked(self.allocation.as_mut_ptr()) }
    }

    pub fn addr(&mut self) -> NonZeroUsize {
        self.as_mut_ptr().addr()
    }
}

impl Deref for PinnedHeapPages {
    type Target = [Page];

    fn deref(&self) -> &Self::Target {
        &self.allocation
    }
}

impl DerefMut for PinnedHeapPages {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.allocation
    }
}

#[cfg(test)]
mod tests {
    use crate::memory::{page::Pages, PAGE_SIZE};

    use super::{Page, PinnedHeapPages};

    #[test_case]
    fn zero_page() {
        let page = Page::zero();
        assert_eq!(page.0, [0; PAGE_SIZE]);
    }

    #[test_case]
    fn new() {
        let heap_pages = PinnedHeapPages::new(2);
        assert_eq!(heap_pages.allocation.len(), 2);
    }

    #[test_case]
    fn with_data() {
        let data = [1u8, 2, 3];
        let mut heap_pages = PinnedHeapPages::new(1);
        heap_pages.fill(&data);
        assert_eq!(heap_pages.len(), 1);
        let heap_slice = heap_pages.as_u8_slice();
        assert_eq!(&heap_slice[..3], &data);
        assert_eq!(&heap_slice[3..], [0; PAGE_SIZE - 3])
    }

    #[test_case]
    fn with_more_data() {
        const LENGTH: usize = PAGE_SIZE + 3;
        let data = [42u8; LENGTH];
        let mut heap_pages = PinnedHeapPages::new(2);
        heap_pages.fill(&data);
        assert_eq!(heap_pages.len(), 2);
        let heap_slice = heap_pages.as_u8_slice();
        assert_eq!(&heap_slice[..LENGTH], &data);
        assert_eq!(&heap_slice[LENGTH..], [0; PAGE_SIZE - 3]);
    }

    #[test_case]
    fn as_u8_slice_works() {
        let mut heap_pages = PinnedHeapPages::new(2);
        let u8_slice = heap_pages.as_u8_slice();
        assert_eq!(u8_slice.len(), PAGE_SIZE * 2);
        assert_eq!(
            u8_slice.as_ptr() as *const Page,
            heap_pages.allocation.as_ptr()
        );
    }
}
