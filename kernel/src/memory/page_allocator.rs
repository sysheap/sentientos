use core::ptr::{null_mut, NonNull};

use crate::{
    klibc::{util::align_up, Mutex},
    print, println,
};

pub const PAGE_SIZE: usize = 4096;
type Page = [u8; PAGE_SIZE];

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

        println!("Page allocator initalized");
        println!("Metadata start:\t\t{:p}", self.metadata);
        println!("Heap start:\t\t{:p}", self.heap);
        println!("Number of pages:\t{}\n", self.number_of_pages);
    }

    fn page_idx_to_page_pointer(&self, page_index: usize) -> PagePointer {
        assert!(page_index < self.number_of_pages);
        unsafe { PagePointer::new(self.heap.add(page_index)) }
    }

    fn page_pointer_to_page_idx(&self, page_pointer: &PagePointer) -> usize {
        let distance = page_pointer.addr.as_ptr() as usize - self.heap as usize;
        assert!(distance % 4096 == 0);
        distance / 4096
    }

    fn zalloc(&self, number_of_pages_requested: usize) -> Option<PagePointer> {
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
                let mut page_pointer = self.page_idx_to_page_pointer(idx);
                page_pointer.zero();
                return Some(page_pointer);
            }
        }
        None
    }

    fn dealloc(&self, page: PagePointer) {
        let mut idx = self.page_pointer_to_page_idx(&page);
        unsafe {
            while *self.metadata.add(idx) != PageStatus::Last {
                *self.metadata.add(idx) = PageStatus::Free;
                idx += 1;
            }
            *self.metadata.add(idx) = PageStatus::Free;
        }
    }

    fn dump(&self) {
        println!("###############");
        println!("Page allocator dump");
        println!("Metadata start:\t\t{:p}", self.metadata);
        println!("Heap start:\t\t{:p}", self.heap);
        println!("Number of pages:\t{}", self.number_of_pages);
        for idx in 0..self.number_of_pages {
            let status = unsafe {
                match *self.metadata.add(idx) {
                    PageStatus::Free => "F",
                    PageStatus::Used => "U",
                    PageStatus::Last => "L",
                }
            };
            print!("{} ", status);

            if (idx + 1) % 80 == 0 {
                print!("\n");
            }
        }
        println!("\n###############");
    }
}

#[derive(Debug)]
pub struct PagePointer {
    addr: NonNull<Page>,
}

impl PagePointer {
    fn new(free_page: *mut Page) -> Self {
        let addr = NonNull::new(free_page).unwrap();
        Self { addr }
    }

    pub fn addr(&self) -> NonNull<[u8; PAGE_SIZE]> {
        self.addr
    }

    pub fn zero(&mut self) {
        unsafe {
            self.addr.as_mut().fill(0);
        }
    }
}

impl From<usize> for PagePointer {
    fn from(pointer: usize) -> Self {
        assert_eq!(pointer % PAGE_SIZE, 0);
        assert!(pointer != 0);
        unsafe {
            Self {
                addr: NonNull::new_unchecked(pointer as *mut Page),
            }
        }
    }
}

static PAGE_ALLOCATOR: Mutex<PageAllocator> = Mutex::new(PageAllocator::new());

pub fn init(heap_start: usize, heap_size: usize) {
    PAGE_ALLOCATOR.lock().init(heap_start, heap_size);
}

pub fn zalloc(number_of_pages: usize) -> Option<PagePointer> {
    PAGE_ALLOCATOR.lock().zalloc(number_of_pages)
}

pub fn dealloc(page: PagePointer) {
    PAGE_ALLOCATOR.lock().dealloc(page);
}

pub fn dump() {
    PAGE_ALLOCATOR.lock().dump();
}
