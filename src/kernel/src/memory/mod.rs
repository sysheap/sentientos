use core::{
    mem::{transmute, MaybeUninit},
    slice::from_raw_parts_mut,
};

use common::mutex::Mutex;

use self::page_allocator::PageAllocator;

pub mod allocated_pages;
pub mod heap;
mod page_allocator;
pub mod page_tables;

pub use page_allocator::PAGE_SIZE;

static PAGE_ALLOCATOR: Mutex<PageAllocator> = Mutex::new(PageAllocator::new());

pub fn init_page_allocator(heap_start: usize, heap_size: usize) {
    let memory = unsafe { from_raw_parts_mut(heap_start as *mut MaybeUninit<u8>, heap_size) };
    for elem in memory.iter_mut() {
        elem.write(0);
    }
    let initialized_memory = unsafe { transmute::<&mut [MaybeUninit<u8>], &mut [u8]>(memory) };
    PAGE_ALLOCATOR.lock().init(initialized_memory);
}
