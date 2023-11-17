use core::{
    marker::PhantomData,
    ptr::{null_mut, NonNull},
    slice,
};

use common::mutex::Mutex;

use crate::{debug, info, klibc::util::align_up};

pub const PAGE_SIZE: usize = 4096;
pub type Page = [u8; PAGE_SIZE];

#[repr(u8)]
#[derive(PartialEq, Eq)]
enum PageStatus {
    Free,
    Used,
    Last,
}

struct PageAllocator {
    metadata: *mut PageStatus,
    heap: *mut Page,
    total_heap_pages: usize,
}

impl PageAllocator {
    const fn new() -> Self {
        Self {
            metadata: null_mut(),
            heap: null_mut(),
            total_heap_pages: 0,
        }
    }

    fn init(&mut self, heap_start: usize, heap_size: usize) {
        let number_of_pages = heap_size / (PAGE_SIZE + 1); // We need one byte per page as metadata

        self.metadata = heap_start as *mut PageStatus;
        self.heap = align_up(heap_start + number_of_pages, PAGE_SIZE) as *mut Page;
        self.total_heap_pages = number_of_pages;

        assert!(self.metadata as usize % PAGE_SIZE == 0);
        assert!(self.heap as usize % PAGE_SIZE == 0);
        assert!(self.heap as usize - self.metadata as usize >= number_of_pages);
        assert!((self.heap as usize + (number_of_pages * PAGE_SIZE)) <= (heap_start + heap_size));

        for idx in 0..number_of_pages {
            unsafe {
                *self.metadata.add(idx) = PageStatus::Free;
            }
        }

        info!("Page allocator initalized");
        info!("Metadata start:\t\t{:p}", self.metadata);
        info!("Heap start:\t\t{:p}", self.heap);
        info!("Number of pages:\t{}\n", self.total_heap_pages);
    }

    fn page_idx_to_pointer(&self, page_index: usize) -> NonNull<Page> {
        assert!(page_index < self.total_heap_pages);
        unsafe { NonNull::new(self.heap.add(page_index)).unwrap() }
    }

    fn page_pointer_to_page_idx<T: PageDropper>(&self, page_pointer: &AllocatedPages<T>) -> usize {
        let distance = page_pointer.ptr.as_ptr() as usize - self.heap as usize;
        assert!(distance % 4096 == 0);
        distance / 4096
    }

    fn alloc(&self, number_of_pages_requested: usize) -> Option<NonNull<Page>> {
        (0..self.total_heap_pages)
            .find(|&idx| self.is_range_free(idx, number_of_pages_requested))
            .map(|start_idx| {
                self.mark_range_as_used(start_idx, number_of_pages_requested);
                self.page_idx_to_pointer(start_idx)
            })
    }

    fn is_range_free(&self, start_idx: usize, length: usize) -> bool {
        start_idx + length <= self.total_heap_pages
            && (start_idx..start_idx + length)
                .all(|idx| unsafe { *self.metadata.add(idx) == PageStatus::Free })
    }

    fn mark_range_as_used(&self, start_idx: usize, length: usize) {
        for idx in start_idx..start_idx + length {
            let status = if idx == start_idx + length - 1 {
                PageStatus::Last
            } else {
                PageStatus::Used
            };
            unsafe {
                *self.metadata.add(idx) = status;
            }
        }
    }

    fn dealloc(&self, page: &mut AllocatedPages<Ephemeral>) {
        let mut idx = self.page_pointer_to_page_idx(page);
        unsafe {
            while *self.metadata.add(idx) != PageStatus::Last {
                *self.metadata.add(idx) = PageStatus::Free;
                idx += 1;
            }
            *self.metadata.add(idx) = PageStatus::Free;
        }
        page.number_of_pages = 0;
        page.ptr = NonNull::dangling();
    }

    fn dump(&self) {
        debug!("###############");
        debug!("Page allocator dump");
        debug!("Metadata start:\t\t{:p}", self.metadata);
        debug!("Heap start:\t\t{:p}", self.heap);
        debug!("Number of pages:\t{}", self.total_heap_pages);
        for idx in 0..self.total_heap_pages {
            let status = unsafe {
                match *self.metadata.add(idx) {
                    PageStatus::Free => "F",
                    PageStatus::Used => "U",
                    PageStatus::Last => "L",
                }
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

pub fn init(heap_start: usize, heap_size: usize) {
    PAGE_ALLOCATOR.lock().init(heap_start, heap_size);
}

#[allow(dead_code)]
pub fn dump() {
    PAGE_ALLOCATOR.lock().dump();
}
