use core::alloc::GlobalAlloc;

struct Heap;

#[global_allocator]
static OS_HEAP: Heap = Heap {};

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}
