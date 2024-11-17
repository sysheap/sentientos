use core::{mem::MaybeUninit, ops::Range, ptr::NonNull, slice::from_raw_parts_mut};

use common::mutex::Mutex;

use self::{
    page::Page,
    page_allocator::{MetadataPageAllocator, PageAllocator},
};

pub mod heap;
pub mod linker_information;
pub mod page;
mod page_allocator;
pub mod page_tables;
mod runtime_mappings;

pub use page::PAGE_SIZE;

pub use runtime_mappings::{initialize_runtime_mappings, RuntimeMapping};

static PAGE_ALLOCATOR: Mutex<MetadataPageAllocator> = Mutex::new(MetadataPageAllocator::new());

pub struct StaticPageAllocator;

impl PageAllocator for StaticPageAllocator {
    fn alloc(number_of_pages_requested: usize) -> Option<Range<NonNull<Page>>> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages_requested)
    }

    fn dealloc(page: NonNull<Page>) -> usize {
        PAGE_ALLOCATOR.lock().dealloc(page)
    }
}

pub fn init_page_allocator(
    heap_start: usize,
    heap_size: usize,
    reserved_areas: &[Range<*const u8>],
) {
    let memory = unsafe { from_raw_parts_mut(heap_start as *mut MaybeUninit<u8>, heap_size) };
    PAGE_ALLOCATOR.lock().init(memory, reserved_areas);
}

pub fn used_heap_pages() -> usize {
    PAGE_ALLOCATOR.lock().used_heap_pages()
}

pub fn total_heap_pages() -> usize {
    PAGE_ALLOCATOR.lock().total_heap_pages()
}

pub fn is_area_reserved<T>(range: &Range<*const T>) -> bool {
    PAGE_ALLOCATOR.lock().is_area_reserved(range)
}
