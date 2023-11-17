use core::{
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
    Free = 1 << 0,
    Used = 1 << 1,
    Last = 1 << 2,
}

struct PageAllocator {
    metadata: *mut PageStatus,
    heap: *mut Page,
    number_of_pages: usize,
}

impl PageAllocator {
    const fn new() -> Self {
        Self {
            metadata: null_mut(),
            heap: null_mut(),
            number_of_pages: 0,
        }
    }

    fn init(&mut self, heap_start: usize, heap_size: usize) {
        let number_of_pages = heap_size / (PAGE_SIZE + 1); // We need one byte per page as metadata

        self.metadata = heap_start as *mut PageStatus;
        self.heap = align_up(heap_start + number_of_pages, PAGE_SIZE) as *mut Page;
        self.number_of_pages = number_of_pages;

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
        info!("Number of pages:\t{}\n", self.number_of_pages);
    }

    fn page_idx_to_pointer(&self, page_index: usize) -> NonNull<Page> {
        assert!(page_index < self.number_of_pages);
        unsafe { NonNull::new(self.heap.add(page_index)).unwrap() }
    }

    fn page_pointer_to_page_idx(&self, page_pointer: &AllocatedPages) -> usize {
        let distance = page_pointer.ptr.as_ptr() as usize - self.heap as usize;
        assert!(distance % 4096 == 0);
        distance / 4096
    }

    fn alloc(&self, number_of_pages_requested: usize) -> Option<NonNull<Page>> {
        'outer: for idx in 0..self.number_of_pages {
            unsafe {
                // Check if this page is free and also if we have enough pages left where we can check consecutiveness
                if *self.metadata.add(idx) != PageStatus::Free
                    || (idx + number_of_pages_requested) > self.number_of_pages
                {
                    continue;
                }
                for consecutive_idx in idx..(idx + number_of_pages_requested) {
                    if *self.metadata.add(consecutive_idx) != PageStatus::Free {
                        continue 'outer;
                    }
                }
                // Got it! We have enough free consecutive pages. Mark the as used
                for mark_idx in idx..(idx + number_of_pages_requested - 1) {
                    *self.metadata.add(mark_idx) = PageStatus::Used;
                }
                *self.metadata.add(idx + number_of_pages_requested - 1) = PageStatus::Last;
                let page_pointer = self.page_idx_to_pointer(idx);
                return Some(page_pointer);
            }
        }
        None
    }

    fn dealloc(&self, page: &mut AllocatedPages) {
        let mut idx = self.page_pointer_to_page_idx(&page);
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
        debug!("Number of pages:\t{}", self.number_of_pages);
        for idx in 0..self.number_of_pages {
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

#[derive(Debug)]
pub struct EthernalPages {
    ptr: NonNull<Page>,
    number_of_pages: usize,
}

impl EthernalPages {
    pub fn zalloc(number_of_pages: usize) -> Option<Self> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages).map(|ptr| {
            let mut allocated_page = Self {
                ptr,
                number_of_pages,
            };
            allocated_page.zero();
            allocated_page
        })
    }

    pub fn zero(&mut self) {
        for offset in 0..self.number_of_pages {
            unsafe {
                self.ptr.as_ptr().add(offset).as_mut().unwrap().fill(0);
            }
        }
    }

    pub fn addr(&self) -> NonNull<Page> {
        self.ptr
    }
}

#[derive(Debug)]
pub struct AllocatedPages {
    ptr: NonNull<Page>,
    number_of_pages: usize,
}

impl AllocatedPages {
    pub fn zalloc(number_of_pages: usize) -> Option<Self> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages).map(|ptr| {
            let mut allocated_page = Self {
                ptr,
                number_of_pages,
            };
            allocated_page.zero();
            allocated_page
        })
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

impl Drop for AllocatedPages {
    fn drop(&mut self) {
        static mut FOO: usize = 0;
        debug!("Drop allocated page at {:p}", self.ptr.as_ptr());
        unsafe {
            FOO += 1;
            if FOO < 10 {
                return;
            }
        }
        PAGE_ALLOCATOR.lock().dealloc(self);
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
