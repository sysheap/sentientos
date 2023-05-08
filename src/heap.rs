use core::alloc::GlobalAlloc;

use crate::println;

struct Heap {
    start: *mut u8,
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
            start: 0 as *mut u8,
            size: 0,
        }
    }

    fn init(&mut self, start: *mut u8, size: usize) {
        self.start = start;
        self.size = size;
    }
}

pub fn init() {
    unsafe {
        OS_HEAP.init(HEAP_START as *mut u8, HEAP_SIZE);
        println!(
            "Heap initialized! Start: {:p}; Size: 0x{:x}",
            OS_HEAP.start, OS_HEAP.size
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
