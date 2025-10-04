use alloc::boxed::Box;
use common::util::copy_slice;
use core::ops::{Add, Deref, DerefMut};

pub const PAGE_SIZE: usize = 4096;

pub struct Pages(pub usize);

impl Add<Pages> for usize {
    type Output = usize;

    fn add(self, rhs: Pages) -> Self::Output {
        (rhs.0 * PAGE_SIZE) + self
    }
}

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

pub trait PagesAsSlice {
    fn as_u8_slice(&mut self) -> &mut [u8];
}

impl PagesAsSlice for [Page] {
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

    pub fn new_pages(pages: Pages) -> Self {
        Self::new(pages.0)
    }

    pub fn fill(&mut self, data: &[u8], offset: usize) {
        copy_slice(data, &mut self.as_u8_slice()[offset..offset + data.len()]);
    }

    pub fn addr(&self) -> usize {
        self.allocation.as_ptr() as usize
    }

    pub fn size(&self) -> usize {
        self.allocation.len() * PAGE_SIZE
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
    use crate::memory::{PAGE_SIZE, page::PagesAsSlice};

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
        heap_pages.fill(&data, 0);
        assert_eq!(heap_pages.len(), 1);
        let heap_slice = heap_pages.as_u8_slice();
        assert_eq!(&heap_slice[..3], &data);
        assert_eq!(&heap_slice[3..], [0; PAGE_SIZE - 3])
    }

    #[test_case]
    fn with_offset() {
        let data = [1u8, 2, 3];
        let mut heap_pages = PinnedHeapPages::new(1);
        heap_pages.fill(&data, 3);
        assert_eq!(heap_pages.len(), 1);
        let heap_slice = heap_pages.as_u8_slice();
        assert_eq!(&heap_slice[..3], &[0, 0, 0]);
        assert_eq!(&heap_slice[3..6], &data);
        assert_eq!(&heap_slice[6..], [0; PAGE_SIZE - 6])
    }

    #[test_case]
    fn with_more_data() {
        const LENGTH: usize = PAGE_SIZE + 3;
        let data = [42u8; LENGTH];
        let mut heap_pages = PinnedHeapPages::new(2);
        heap_pages.fill(&data, 0);
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
