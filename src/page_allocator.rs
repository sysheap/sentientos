use core::ptr::NonNull;

use crate::{println, util};

const PAGE_SIZE: usize = 4096;

struct PageAllocator {
    free_list: Option<NonNull<FreePage>>,
}

impl PageAllocator {
    const fn new() -> Self {
        Self { free_list: None }
    }

    fn init(&mut self, heap_start: usize, heap_size: usize) {
        let heap_start_aligned = util::align_up(heap_start, PAGE_SIZE);
        let heap_end_aligned = util::align_down(heap_start + heap_size, PAGE_SIZE);

        let mut current = NonNull::new(heap_start_aligned as *const FreePage as *mut FreePage)
            .expect("Heap start must not be nul.");
        self.free_list = Some(current);

        loop {
            unsafe {
                let next_free_page_addr = current.as_ptr().byte_add(PAGE_SIZE);

                if next_free_page_addr.addr() >= heap_end_aligned {
                    current.as_mut().next = None;
                    break;
                }

                let next_free_page = NonNull::new(next_free_page_addr);
                current.as_mut().next = next_free_page;
                current = next_free_page.expect("Next free page must be not null.");
            }
        }

        println!(
            "Page allocator initialized! (Start: 0x{:x}, End: 0x{:x})\n",
            heap_start_aligned, heap_end_aligned
        );
    }

    fn zalloc(&mut self) -> Option<Page> {
        let page = match self.free_list {
            None => return None,
            Some(page) => page,
        };
        unsafe {
            self.free_list = page.as_ref().next;
        }
        Some(Page::new(page))
    }

    fn dealloc(&mut self, page: Page) {
        let mut free_page: NonNull<FreePage> = page.addr.cast();
        unsafe {
            free_page.as_mut().next = self.free_list;
            self.free_list = Some(free_page);
        }
    }
}

struct FreePage {
    next: Option<NonNull<FreePage>>,
}

#[derive(Debug)]
pub struct Page {
    addr: NonNull<[u8; PAGE_SIZE]>,
}

impl Page {
    fn new(free_page: NonNull<FreePage>) -> Self {
        let mut addr: NonNull<[u8; PAGE_SIZE]> = free_page.cast();
        unsafe {
            addr.as_mut().fill(0);
        }
        Self { addr }
    }

    pub fn addr(&self) -> NonNull<[u8; PAGE_SIZE]> {
        self.addr
    }
}

static mut PAGE_ALLOCATOR: PageAllocator = PageAllocator::new();

pub fn init(heap_start: usize, heap_size: usize) {
    unsafe {
        PAGE_ALLOCATOR.init(heap_start, heap_size);
    }
}

pub fn zalloc() -> Option<Page> {
    unsafe { PAGE_ALLOCATOR.zalloc() }
}

pub fn dealloc(page: Page) {
    unsafe {
        PAGE_ALLOCATOR.dealloc(page);
    }
}
