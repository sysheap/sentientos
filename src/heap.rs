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
static OS_HEAP: Heap = unsafe { Heap::new(HEAP_START as *mut u8, HEAP_SIZE) };

impl Heap {
    const unsafe fn new(start: *mut u8, size: usize) -> Self {
        Self { start, size }
    }
}

// TODO: This is unsafe! Our heap is not thread safe yet.
unsafe impl Sync for Heap {}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        println!("Start: {:p}; Size: {:x}", self.start, self.size);
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}
