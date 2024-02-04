use crate::{debug, memory::PAGE_SIZE};
use core::{fmt::Debug, ops::Range, ptr::NonNull};

use super::page::Page;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
enum PageStatus {
    Free,
    Used,
    Last,
}

pub(super) struct MetadataPageAllocator<'a> {
    metadata: &'a mut [PageStatus],
    pages: Range<*mut Page>,
}

impl<'a> Debug for MetadataPageAllocator<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageAllocator")
            .field("metadata", &self.metadata.as_ptr())
            .field("pages", &self.pages)
            .finish()
    }
}

impl<'a> MetadataPageAllocator<'a> {
    pub(super) fn new(memory: &'a mut [u8]) -> Self {
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

        let pages = heap.as_mut_ptr_range();

        debug!("Page allocator initalized");
        debug!("Metadata start:\t\t{:p}", metadata);
        debug!("Heap start:\t\t{:p}", pages.start);
        debug!("Number of pages:\t{}\n", metadata.len());

        Self { metadata, pages }
    }

    fn total_heap_pages(&self) -> usize {
        self.metadata.len()
    }

    fn page_idx_to_pointer(&self, page_index: usize) -> NonNull<Page> {
        unsafe { NonNull::new(self.pages.start.add(page_index)).unwrap() }
    }

    fn page_pointer_to_page_idx(&self, page: NonNull<Page>) -> usize {
        let heap_start = self.pages.start;
        let heap_end = self.pages.end;
        let page_ptr = page.as_ptr();
        assert!(page_ptr >= heap_start);
        assert!(page_ptr < heap_end);
        assert!(page_ptr.is_aligned());
        let offset = unsafe { page_ptr.offset_from(heap_start) };
        offset as usize
    }

    pub fn alloc(&mut self, number_of_pages_requested: usize) -> Option<Range<NonNull<Page>>> {
        let total_pages = self.total_heap_pages();
        if number_of_pages_requested > total_pages {
            return None;
        }
        (0..=(self.total_heap_pages() - number_of_pages_requested))
            .find(|&idx| self.is_range_free(idx, number_of_pages_requested))
            .map(|start_idx| {
                self.mark_range_as_used(start_idx, number_of_pages_requested);
                self.page_idx_to_pointer(start_idx)
                    ..self.page_idx_to_pointer(start_idx + number_of_pages_requested)
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

pub trait PageAllocator {
    fn alloc(number_of_pages_requested: usize) -> Option<Range<NonNull<Page>>>;
    fn dealloc(page: NonNull<Page>);
}

#[cfg(test)]
mod tests {
    use core::ptr::addr_of_mut;

    use crate::memory::page_allocator::PageStatus;

    use super::{MetadataPageAllocator, PAGE_SIZE};

    static mut PAGE_ALLOC_MEMORY: [u8; PAGE_SIZE * 8] = [0; PAGE_SIZE * 8];

    fn create_allocator() -> MetadataPageAllocator<'static> {
        unsafe { MetadataPageAllocator::new(&mut *addr_of_mut!(PAGE_ALLOC_MEMORY)) }
    }

    #[test_case]
    fn clean_start() {
        let allocator = create_allocator();
        assert!(allocator.metadata.iter().all(|s| *s == PageStatus::Free));
    }

    #[test_case]
    fn exhaustion_allocation() {
        let mut allocator = create_allocator();
        let number_of_pages = allocator.total_heap_pages();
        let _pages = allocator.alloc(number_of_pages).unwrap();
        assert!(allocator.alloc(1).is_none());
        let allocator = allocator;
        let (last, all_metadata_except_last) = allocator.metadata.split_last().unwrap();
        assert!(all_metadata_except_last
            .iter()
            .all(|s| *s == PageStatus::Used));
        assert_eq!(*last, PageStatus::Last);
    }

    #[test_case]
    fn beyond_capacity() {
        let mut allocator = create_allocator();
        let number_of_pages = allocator.total_heap_pages();
        let pages = allocator.alloc(number_of_pages + 1);
        assert!(pages.is_none());
    }

    #[test_case]
    fn all_single_allocations() {
        let mut allocator = create_allocator();
        let number_of_pages = allocator.total_heap_pages();
        for _ in 0..number_of_pages {
            assert!(allocator.alloc(1).is_some());
        }
        assert!(allocator.alloc(1).is_none());
    }

    #[test_case]
    fn metadata_integrity() {
        let mut allocator = create_allocator();
        let page1 = allocator.alloc(1).unwrap();
        assert_eq!(allocator.metadata[0], PageStatus::Last);
        assert!(allocator.metadata[1..]
            .iter()
            .all(|s| *s == PageStatus::Free));
        let page2 = allocator.alloc(2).unwrap();
        assert_eq!(
            allocator.metadata[..3],
            [PageStatus::Last, PageStatus::Used, PageStatus::Last]
        );
        assert!(allocator.metadata[3..]
            .iter()
            .all(|s| *s == PageStatus::Free));
        let page3 = allocator.alloc(3).unwrap();
        assert_eq!(
            allocator.metadata[..6],
            [
                PageStatus::Last,
                PageStatus::Used,
                PageStatus::Last,
                PageStatus::Used,
                PageStatus::Used,
                PageStatus::Last
            ]
        );
        assert!(allocator.metadata[6..]
            .iter()
            .all(|s| *s == PageStatus::Free),);
        allocator.dealloc(page2.start);
        assert_eq!(
            allocator.metadata[..6],
            [
                PageStatus::Last,
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Used,
                PageStatus::Used,
                PageStatus::Last
            ]
        );
        allocator.dealloc(page1.start);
        assert_eq!(
            allocator.metadata[..6],
            [
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Free,
                PageStatus::Used,
                PageStatus::Used,
                PageStatus::Last
            ]
        );
        allocator.dealloc(page3.start);
        assert_eq!(
            allocator.metadata[..6],
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
}
