use core::alloc::GlobalAlloc;

use crate::println;

struct Heap {
    free_list: *mut FreeBlock,
}

#[repr(packed)]
struct FreeBlock {
    metadata: FreeMetadata,
    data: u8,
}

#[repr(packed)]
struct FreeMetadata {
    next: *mut FreeBlock,
    size: usize,
}

extern "C" {
    static mut HEAP_START: usize;
    static mut HEAP_SIZE: usize;
}

#[global_allocator]
static mut OS_HEAP: Heap = Heap::new();

impl Heap {
    const fn new() -> Self {
        Self {
            free_list: core::ptr::null_mut(),
        }
    }

    fn init(&mut self, start: *mut u8, size: usize) {
        let align_start = align_to(start as usize);
        let difference = align_start - start as usize;
        let size = size - difference;

        let free_block: &mut FreeBlock = unsafe { &mut *(align_start as *mut FreeBlock) };

        free_block.metadata.next = core::ptr::null_mut();
        free_block.metadata.size = size;

        self.free_list = free_block as *const FreeBlock as *mut FreeBlock;
    }
}

fn align_to(value: usize) -> usize {
    let remainder = value % 8;
    if remainder == 0 {
        value
    } else {
        value + 8 - remainder
    }
}

pub fn init() {
    unsafe {
        OS_HEAP.init(HEAP_START as *mut u8, HEAP_SIZE);
        println!(
            "Heap initialized! (Start: 0x{:x} Size: 0x{:x}",
            HEAP_START, HEAP_SIZE
        );
    }
}

// TODO: This is unsafe! Our heap is not thread safe yet.
unsafe impl Sync for Heap {}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}
