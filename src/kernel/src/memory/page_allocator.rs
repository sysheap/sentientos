use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice::{self, from_raw_parts_mut},
};

use common::mutex::Mutex;

use crate::{debug, info};

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
#[derive(PartialEq, Eq)]
enum PageStatus {
    Free,
    Used,
    Last,
}

struct PageAllocator {
    metadata: &'static mut [PageStatus],
    pages: &'static mut [Page],
}

impl PageAllocator {
    const fn new() -> Self {
        Self {
            metadata: &mut [],
            pages: &mut [],
        }
    }

    fn init(&mut self, memory: &'static mut [u8]) {
        let heap_size = memory.len();
        let number_of_heap_pages = heap_size / (PAGE_SIZE + 1); // We need one byte per page as metadata

        let (metadata, heap) = memory.split_at_mut(number_of_heap_pages);

        let (begin, metadata, end) = unsafe { metadata.align_to_mut::<PageStatus>() };
        assert!(begin.len() == 0);
        assert!(end.len() == 0);

        let (_begin, heap, _end) = unsafe { heap.align_to_mut::<Page>() };
        assert!(metadata.len() <= heap.len());
        assert!(heap[0].as_ptr() as usize % PAGE_SIZE == 0);

        let size_metadata = core::mem::size_of_val(metadata);
        let size_heap = core::mem::size_of_val(heap);
        assert!(size_metadata + size_heap <= heap_size);

        self.metadata = metadata;
        self.pages = heap;

        self.metadata.iter_mut().for_each(|x| *x = PageStatus::Free);

        info!("Page allocator initalized");
        info!("Metadata start:\t\t{:p}", self.metadata);
        info!("Heap start:\t\t{:p}", self.pages);
        info!("Number of pages:\t{}\n", self.total_heap_pages());
    }

    fn total_heap_pages(&self) -> usize {
        self.metadata.len()
    }

    fn page_idx_to_pointer(&mut self, page_index: usize) -> NonNull<Page> {
        let page = &mut self.pages[page_index];
        NonNull::new(page as *mut _).unwrap()
    }

    fn page_pointer_to_page_idx<T: PageDropper>(&self, page: &AllocatedPages<T>) -> usize {
        let heap_start = self.pages.as_ptr();
        let heap_end = self
            .pages
            .last()
            .map(|x| x.as_ptr() as *const _)
            .unwrap_or(heap_start);
        let page_ptr = page.addr().as_ptr() as *const _;
        assert!(page_ptr >= heap_start && page_ptr < heap_end);
        let offset = page_ptr as usize - heap_start as usize;
        assert!(offset % PAGE_SIZE == 0);
        offset / PAGE_SIZE
    }

    fn alloc(&mut self, number_of_pages_requested: usize) -> Option<NonNull<Page>> {
        (0..self.total_heap_pages())
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

    fn dealloc(&mut self, page: &mut AllocatedPages<Ephemeral>) {
        let mut idx = self.page_pointer_to_page_idx(page);

        while self.metadata[idx] != PageStatus::Last {
            self.metadata[idx] = PageStatus::Free;
            idx += 1;
        }
        self.metadata[idx] = PageStatus::Free;

        page.number_of_pages = 0;
        page.ptr = NonNull::dangling();
    }

    fn dump(&self) {
        debug!("###############");
        debug!("Page allocator dump");
        debug!("Metadata start:\t\t{:p}", self.metadata);
        debug!("Heap start:\t\t{:p}", self.pages);
        debug!("Number of pages:\t{}", self.total_heap_pages());
        for idx in 0..self.total_heap_pages() {
            let status = match self.metadata[idx] {
                PageStatus::Free => "F",
                PageStatus::Used => "U",
                PageStatus::Last => "L",
            };
            debug!("{} ", status);

            if (idx + 1) % 80 == 0 {
                debug!("\n");
            }
        }
        debug!("\n###############");
    }
}

#[derive(Debug, Default)]
pub struct Ephemeral;
#[derive(Debug, Default)]
pub struct Ethernal;

pub trait PageDropper: Sized {
    fn drop(page: &mut AllocatedPages<Self>);
}

impl PageDropper for Ephemeral {
    fn drop(page: &mut AllocatedPages<Self>) {
        debug!("Drop allocated page at {:p}", page.ptr.as_ptr());
        PAGE_ALLOCATOR.lock().dealloc(page);
    }
}
impl PageDropper for Ethernal {
    fn drop(_page: &mut AllocatedPages<Self>) {}
}

#[derive(Debug)]
pub struct AllocatedPages<Dropper: PageDropper> {
    ptr: NonNull<Page>,
    number_of_pages: usize,
    phantom: PhantomData<Dropper>,
}

impl<Dropper: PageDropper> AllocatedPages<Dropper> {
    pub fn zalloc(number_of_pages: usize) -> Option<Self> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages).map(|ptr| {
            let mut allocated_page = Self::new(ptr, number_of_pages);
            allocated_page.zero();
            allocated_page
        })
    }

    fn new(ptr: NonNull<Page>, number_of_pages: usize) -> Self {
        Self {
            ptr,
            number_of_pages,
            phantom: PhantomData,
        }
    }

    pub fn addr(&self) -> NonNull<Page> {
        self.ptr
    }

    fn u8(&self) -> *mut u8 {
        self.ptr.cast().as_ptr()
    }

    pub fn slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.u8(), self.number_of_pages * PAGE_SIZE) }
    }

    pub fn addr_as_usize(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    pub fn zero(&mut self) {
        for offset in 0..self.number_of_pages {
            unsafe {
                self.ptr.as_ptr().add(offset).as_mut().unwrap().fill(0);
            }
        }
    }
}

impl<Dropper: PageDropper> Drop for AllocatedPages<Dropper> {
    fn drop(&mut self) {
        Dropper::drop(self);
    }
}

static PAGE_ALLOCATOR: Mutex<PageAllocator> = Mutex::new(PageAllocator::new());

pub fn init(heap_start: *mut u8, heap_size: usize) {
    let memory = unsafe { from_raw_parts_mut(heap_start, heap_size) };
    PAGE_ALLOCATOR.lock().init(memory);
}

#[allow(dead_code)]
pub fn dump() {
    PAGE_ALLOCATOR.lock().dump();
}
