use core::{
    cell::OnceCell,
    mem::{transmute, MaybeUninit},
    ops::Range,
    ptr::NonNull,
    slice::from_raw_parts_mut,
};

use common::mutex::Mutex;

use self::{
    page::Page,
    page_allocator::{MetadataPageAllocator, PageAllocator},
};

pub mod heap;
pub mod page;
mod page_allocator;
pub mod page_tables;

pub use page::PAGE_SIZE;

static PAGE_ALLOCATOR: Mutex<OnceCell<MetadataPageAllocator>> = Mutex::new(OnceCell::new());

pub struct StaticPageAllocator;

impl PageAllocator for StaticPageAllocator {
    fn alloc(number_of_pages_requested: usize) -> Option<Range<NonNull<Page>>> {
        PAGE_ALLOCATOR
            .lock()
            .get_mut()
            .expect("PAGE_ALLOCATOR has to be initialized")
            .alloc(number_of_pages_requested)
    }

    fn dealloc(page: NonNull<Page>) {
        PAGE_ALLOCATOR
            .lock()
            .get_mut()
            .expect("PAGE_ALLOCATOR has to be initialized")
            .dealloc(page)
    }
}

pub fn init_page_allocator(heap_start: usize, heap_size: usize) {
    let memory = unsafe { from_raw_parts_mut(heap_start as *mut MaybeUninit<u8>, heap_size) };
    for elem in memory.iter_mut() {
        elem.write(0);
    }
    let initialized_memory = unsafe { transmute::<&mut [MaybeUninit<u8>], &mut [u8]>(memory) };
    PAGE_ALLOCATOR
        .lock()
        .set(MetadataPageAllocator::new(initialized_memory))
        .expect("PAGE_ALLOCATOR has to be uninitialized");
}
