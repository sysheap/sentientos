use crate::debug;
use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
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

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
enum PageStatus {
    Free,
    Used,
    Last,
}

pub(super) struct PageAllocator<'a> {
    metadata: &'a mut [PageStatus],
    pages: &'a mut [Page],
}

impl<'a> Debug for PageAllocator<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageAllocator")
            .field("metadata", &self.metadata.as_ptr())
            .field("pages", &self.pages.as_ptr())
            .finish()
    }
}

impl<'a> PageAllocator<'a> {
    pub(super) const fn new() -> Self {
        Self {
            metadata: &mut [],
            pages: &mut [],
        }
    }

    pub(super) fn init(&mut self, memory: &'a mut [u8]) {
        let heap_size = memory.len();
        let number_of_heap_pages = heap_size / (PAGE_SIZE + 1); // We need one byte per page as metadata

        let (metadata, heap) = memory.split_at_mut(number_of_heap_pages);

        let (begin, metadata, end) = unsafe { metadata.align_to_mut::<PageStatus>() };
        assert!(begin.is_empty());
        assert!(end.is_empty());

        let (_begin, heap, _end) = unsafe { heap.align_to_mut::<Page>() };
        assert!(metadata.len() <= heap.len());
        assert!(heap[0].as_ptr() as usize % PAGE_SIZE == 0);

        let size_metadata = core::mem::size_of_val(metadata);
        let size_heap = core::mem::size_of_val(heap);
        assert!(size_metadata + size_heap <= heap_size);

        metadata.iter_mut().for_each(|x| *x = PageStatus::Free);

        self.metadata = metadata;
        self.pages = heap;

        debug!("Page allocator initalized");
        debug!("Metadata start:\t\t{:p}", self.metadata);
        debug!("Heap start:\t\t{:p}", self.pages);
        debug!("Number of pages:\t{}\n", self.total_heap_pages());
    }

    fn total_heap_pages(&self) -> usize {
        self.metadata.len()
    }

    fn page_idx_to_pointer(&mut self, page_index: usize) -> NonNull<Page> {
        let page = &mut self.pages[page_index];
        NonNull::new(page as *mut _).unwrap()
    }

    fn page_pointer_to_page_idx(&self, page: NonNull<Page>) -> usize {
        let heap_start = self.pages.as_ptr();
        let heap_end = self
            .pages
            .last()
            .map(|x| x.as_ptr() as *const _)
            .unwrap_or(heap_start);
        let page_ptr = page.as_ptr() as *const _;
        assert!(page_ptr >= heap_start);
        assert!(page_ptr < heap_end);
        let offset = page_ptr as usize - heap_start as usize;
        assert!(offset % PAGE_SIZE == 0);
        offset / PAGE_SIZE
    }

    pub fn alloc(&mut self, number_of_pages_requested: usize) -> Option<NonNull<Page>> {
        (0..self.total_heap_pages() - number_of_pages_requested)
            .find(|&idx| self.is_range_free(idx, number_of_pages_requested))
            .map(|start_idx| {
                self.mark_range_as_used(start_idx, number_of_pages_requested);
                self.page_idx_to_pointer(start_idx)
            })
    }

    fn is_range_free(&self, start_idx: usize, length: usize) -> bool {
        (start_idx..start_idx + length).all(|idx| self.metadata[idx] == PageStatus::Free)
    }

    fn mark_range_as_used(&mut self, start_idx: usize, length: usize) {
        for idx in start_idx..start_idx + length {
            let status = if idx == start_idx + length - 1 {
                PageStatus::Last
            } else {
                PageStatus::Used
            };

            self.metadata[idx] = status;
        }
    }

    pub fn dealloc(&mut self, page: NonNull<Page>) {
        let mut idx = self.page_pointer_to_page_idx(page);

        while self.metadata[idx] != PageStatus::Last {
            self.metadata[idx] = PageStatus::Free;
            idx += 1;
        }
        self.metadata[idx] = PageStatus::Free;
    }
}

#[cfg(test)]
mod tests {
    use common::mutex::Mutex;

    use crate::memory::{
        allocated_pages::{AllocatedPages, Ephemeral, Ethernal, WhichAllocator},
        page_allocator::PageStatus,
    };

    use super::{PageAllocator, PAGE_SIZE};

    static mut PAGE_ALLOC_MEMORY: [u8; PAGE_SIZE * 5000] = [0; PAGE_SIZE * 5000];
    static PAGE_ALLOC: Mutex<PageAllocator> = Mutex::new(PageAllocator::new());

    struct TestAllocator;
    impl WhichAllocator for TestAllocator {
        fn allocate(number_of_pages: usize) -> Option<core::ptr::NonNull<super::Page>> {
            PAGE_ALLOC.lock().alloc(number_of_pages)
        }

        fn deallocate(pages: core::ptr::NonNull<super::Page>) {
            PAGE_ALLOC.lock().dealloc(pages)
        }
    }

    fn init_allocator() {
        unsafe {
            PAGE_ALLOC.lock().init(&mut PAGE_ALLOC_MEMORY);
        }
    }

    #[test_case]
    fn clean_start() {
        init_allocator();
        assert!(PAGE_ALLOC
            .lock()
            .metadata
            .iter()
            .all(|s| *s == PageStatus::Free));
    }

    #[test_case]
    fn exhaustion_allocation() {
        init_allocator();
        let number_of_pages = PAGE_ALLOC.lock().total_heap_pages();
        let _pages = AllocatedPages::<Ephemeral, TestAllocator>::zalloc(number_of_pages).unwrap();
        let allocator = PAGE_ALLOC.lock();
        let (last, all_metadata_except_last) = allocator.metadata.split_last().unwrap();
        assert!(all_metadata_except_last
            .iter()
            .all(|s| *s == PageStatus::Used));
        assert_eq!(*last, PageStatus::Last);
    }

    #[test_case]
    fn metadata_integrity() {
        init_allocator();
        let page1 = AllocatedPages::<Ephemeral, TestAllocator>::zalloc(1).unwrap();
        assert_eq!(PAGE_ALLOC.lock().metadata[0], PageStatus::Last);
        assert!(PAGE_ALLOC.lock().metadata[1..]
            .iter()
            .all(|s| *s == PageStatus::Free));
        let page2 = AllocatedPages::<Ephemeral, TestAllocator>::zalloc(2).unwrap();
        assert_eq!(
            PAGE_ALLOC.lock().metadata[..3],
            [PageStatus::Last, PageStatus::Used, PageStatus::Last]
        );
        assert!(PAGE_ALLOC.lock().metadata[3..]
            .iter()
            .all(|s| *s == PageStatus::Free));
        let page3 = AllocatedPages::<Ephemeral, TestAllocator>::zalloc(3).unwrap();
        assert_eq!(
            PAGE_ALLOC.lock().metadata[..6],
            [
                PageStatus::Last,
                PageStatus::Used,
                PageStatus::Last,
                PageStatus::Used,
                PageStatus::Used,
                PageStatus::Last
            ]
        );
        assert!(PAGE_ALLOC.lock().metadata[6..]
            .iter()
            .all(|s| *s == PageStatus::Free),);
        drop(page2);
        assert_eq!(
            PAGE_ALLOC.lock().metadata[..6],
            [
                PageStatus::Last,
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Used,
                PageStatus::Used,
                PageStatus::Last
            ]
        );
        drop(page1);
        assert_eq!(
            PAGE_ALLOC.lock().metadata[..6],
            [
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Used,
                PageStatus::Used,
                PageStatus::Last
            ]
        );
        drop(page3);
        assert_eq!(
            PAGE_ALLOC.lock().metadata[..6],
            [
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Free
            ]
        );
    }

    #[test_case]
    fn test_ethernal_pages() {
        init_allocator();
        let ethernal = AllocatedPages::<Ethernal, TestAllocator>::zalloc(1).unwrap();
        drop(ethernal);
        let allocator = PAGE_ALLOC.lock();
        assert_eq!(allocator.metadata[0], PageStatus::Last);
        assert!(allocator.metadata[1..]
            .iter()
            .all(|s| *s == PageStatus::Free));
    }
}
