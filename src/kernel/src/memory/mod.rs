use core::slice::from_raw_parts_mut;

use common::mutex::Mutex;

use self::page_allocator::PageAllocator;

pub mod allocated_pages;
pub mod heap;
mod page_allocator;
pub mod page_tables;

pub use page_allocator::PAGE_SIZE;

static PAGE_ALLOCATOR: Mutex<PageAllocator> = Mutex::new(PageAllocator::new());

pub fn init_page_allocator(heap_start: *mut u8, heap_size: usize) {
    let memory: &'static mut [u8] = unsafe { from_raw_parts_mut(heap_start, heap_size) };
    PAGE_ALLOCATOR.lock().init(memory);
}
