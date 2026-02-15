use crate::{device_tree, info, klibc::Spinlock};

use self::{
    page::Page,
    page_allocator::{MetadataPageAllocator, PageAllocator},
};
use core::{mem::MaybeUninit, ops::Range, ptr::NonNull, slice::from_raw_parts_mut};
use linker_information::LinkerInformation;

pub mod heap;
pub mod linker_information;
pub mod page;
mod page_allocator;
pub mod page_table_entry;
pub mod page_tables;
mod runtime_mappings;

pub use page::PAGE_SIZE;

pub use runtime_mappings::initialize_runtime_mappings;

static PAGE_ALLOCATOR: Spinlock<MetadataPageAllocator> =
    Spinlock::new(MetadataPageAllocator::new());

pub struct StaticPageAllocator;

impl PageAllocator for StaticPageAllocator {
    fn alloc(number_of_pages_requested: usize) -> Option<Range<NonNull<Page>>> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages_requested)
    }

    fn dealloc(page: NonNull<Page>) -> usize {
        PAGE_ALLOCATOR.lock().dealloc(page)
    }
}

#[cfg(miri)]
pub fn heap_size() -> usize {
    crate::memory::PAGE_SIZE
}

#[cfg(not(miri))]
pub fn heap_size() -> usize {
    let memory_node = device_tree::THE
        .root_node()
        .find_node("memory")
        .expect("There must be a memory node");

    let reg = memory_node
        .parse_reg_property()
        .expect("Memory node must have a reg property");

    let ram_end_address = reg.address + reg.size;
    ram_end_address - LinkerInformation::__start_heap()
}

pub fn init_page_allocator(reserved_areas: &[Range<*const u8>]) {
    let heap_start = LinkerInformation::__start_heap();
    let heap_size = heap_size();

    info!("Initializing page allocator");
    info!(
        "Heap Start: {:#x}-{:#x} (size: {:#x} -> {})",
        heap_start,
        heap_start + heap_size,
        heap_size,
        crate::klibc::util::PrintMemorySizeHumanFriendly(heap_size)
    );

    // SAFETY: The heap region [heap_start, heap_start+heap_size) is reserved
    // by the linker script and not used by any other code. MaybeUninit<u8>
    // has no validity requirements.
    let memory = unsafe { from_raw_parts_mut(heap_start as *mut MaybeUninit<u8>, heap_size) };
    PAGE_ALLOCATOR.lock().init(memory, reserved_areas);
}

pub fn used_heap_pages() -> usize {
    PAGE_ALLOCATOR.lock().used_heap_pages()
}

pub fn total_heap_pages() -> usize {
    PAGE_ALLOCATOR.lock().total_heap_pages()
}
