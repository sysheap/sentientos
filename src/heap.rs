use core::{alloc::GlobalAlloc, cell::RefCell};

use crate::println;

struct HeapInner {
    free_list: *mut FreeBlock,
}

struct Heap {
    inner: RefCell<HeapInner>,
    start_addr: *const u8,
    size: usize,
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
    used: usize,
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
            inner: RefCell::new(HeapInner {
                free_list: core::ptr::null_mut(),
            }),
            start_addr: core::ptr::null(),
            size: 0,
        }
    }

    fn init(&mut self, start: *mut u8, size: usize) {
        let align_start = align_to(start as usize);
        let difference = align_start - start as usize;
        let size = size - difference;

        let free_block: &mut FreeBlock = unsafe { &mut *(align_start as *mut FreeBlock) };

        free_block.metadata.next = core::ptr::null_mut();
        free_block.metadata.size = size;
        free_block.metadata.used = 0;

        self.inner.borrow_mut().free_list = free_block as *const FreeBlock as *mut FreeBlock;
        self.start_addr = align_start as *const u8;
        self.size = 0;
    }

    fn dump(&self) {
        println!("Heap DUMP");
        println!("START\t\tEND\t\tSIZE\t\tUSED");

        let mut current_metadata_block = self.start_addr as *const FreeMetadata;

        unsafe {
            while current_metadata_block as usize <= self.start_addr as usize + self.size {
                let start_addr = current_metadata_block;
                let size = (*current_metadata_block).size;
                let used = if (*current_metadata_block).used == 0 {
                    "NO"
                } else {
                    "YES"
                };

                println!(
                    "{:p}\t0x{:x}\t0x{:x}\t{}",
                    start_addr,
                    start_addr as usize + size,
                    size,
                    used
                );

                current_metadata_block = current_metadata_block.byte_add(size);
            }
        }

        // println!("Free blocks\nSTART\t\tEND\t\tSIZE");

        // let mut current_free_block_ptr = self.inner.borrow().free_list;
        // unsafe {
        //     while !current_free_block_ptr.is_null() {
        //         let current_free_block = &*current_free_block_ptr;
        //         let size = current_free_block.metadata.size;
        //         println!(
        //             "{:p}\t0x{:x}\t0x{:x}",
        //             current_free_block_ptr,
        //             current_free_block_ptr as usize + size,
        //             size
        //         );
        //         current_free_block_ptr = current_free_block.metadata.next;
        //     }
        // }
    }

    unsafe fn alloc_impl(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc_impl(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let freeblock = &mut *(ptr.sub(core::mem::size_of::<FreeMetadata>()) as *mut FreeBlock);

        freeblock.metadata.next = self.inner.borrow().free_list;
        freeblock.metadata.used = 0;

        self.inner.borrow_mut().free_list = freeblock as *mut FreeBlock;
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

pub fn dump() {
    unsafe {
        OS_HEAP.dump();
    }
}

pub fn init() {
    unsafe {
        OS_HEAP.init(HEAP_START as *mut u8, HEAP_SIZE);
        println!(
            "Heap initialized! (Start: 0x{:x} Size: 0x{:x})",
            HEAP_START, HEAP_SIZE
        );
    }
}

// TODO: This is unsafe! Our heap is not thread safe yet.
unsafe impl Sync for Heap {}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.alloc_impl(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.dealloc_impl(ptr, layout);
    }
}
