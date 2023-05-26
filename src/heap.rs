use core::{alloc::GlobalAlloc, cell::RefCell};

use crate::println;

const DELIMITER: &str = "######################################################";

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
    data: usize,
}

impl FreeBlock {
    fn get_data_ptr(&self) -> *mut u8 {
        let free_block_ptr = self as *const FreeBlock as *const u8 as *mut u8;
        unsafe { free_block_ptr.byte_add(core::mem::size_of::<FreeMetadata>()) }
    }
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
        self.size = size;
    }

    fn dump(&self) {
        println!("{}", DELIMITER);
        println!("Heap DUMP");
        println!("USED\t\tSTART\t\tEND\t\tSIZE");

        let mut current_metadata_block = self.start_addr as *const FreeMetadata;

        unsafe {
            while (current_metadata_block as usize) < (self.start_addr as usize + self.size) {
                let start_addr = current_metadata_block;
                let size = (*current_metadata_block).size;
                let used = if (*current_metadata_block).used == 0 {
                    "NO"
                } else {
                    "YES"
                };

                println!(
                    "{}\t\t{:p}\t0x{:x}\t0x{:x}",
                    used,
                    start_addr,
                    start_addr as usize + size,
                    size
                );

                current_metadata_block = current_metadata_block.byte_add(size);
            }
        }
        println!("{}", DELIMITER);
    }

    unsafe fn alloc_impl(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut size = core::cmp::max(layout.size(), core::mem::size_of::<FreeBlock>());

        let mut previous_free_block = core::ptr::null_mut();

        let mut free_block = self.inner.borrow_mut().free_list;

        // Find free block which is large enough
        unsafe {
            while !free_block.is_null() {
                let free_block_ref = &*free_block;
                if free_block_ref.metadata.size >= size {
                    break;
                }
                previous_free_block = free_block;
                free_block = free_block_ref.metadata.next;
            }
        }

        if free_block.is_null() {
            return core::ptr::null_mut();
        }

        let free_block_ref = &mut *free_block;

        // Check if the rest of the block is too small to fit and consume it complete
        let remaining_size = free_block_ref.metadata.size - size;

        if remaining_size < core::mem::size_of::<FreeBlock>() {
            size = free_block_ref.metadata.size;
        }

        // Two cases: Hand out block completely or partially reduce it
        if size == free_block_ref.metadata.size {
            free_block_ref.metadata.used = 1;

            if previous_free_block.is_null() {
                self.inner.borrow_mut().free_list = free_block_ref.metadata.next;
            } else {
                (*previous_free_block).metadata.next = free_block_ref.metadata.next;
            }

            free_block_ref.metadata.next = core::ptr::null_mut();

            free_block_ref.get_data_ptr()
        } else if size < free_block_ref.metadata.size {
            let new_block_size = free_block_ref.metadata.size - size;
            free_block_ref.metadata.used = 1;
            free_block_ref.metadata.size = size;

            let new_free_block = &mut *free_block.byte_add(size);
            new_free_block.metadata.used = 0;
            new_free_block.metadata.size = new_block_size;
            new_free_block.metadata.next = free_block_ref.metadata.next;

            free_block_ref.metadata.next = core::ptr::null_mut();

            if previous_free_block.is_null() {
                self.inner.borrow_mut().free_list = new_free_block;
            } else {
                (*previous_free_block).metadata.next = new_free_block;
            }

            free_block_ref.get_data_ptr()
        } else {
            panic!("size is larger than free block.");
        }
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
        let size = core::cmp::max(layout.size(), core::mem::size_of::<FreeBlock>());

        println!(
            "BEFORE ALLOC: 0x{:x} (Original: 0x{:x})",
            size,
            layout.size()
        );
        self.dump();

        let ptr = self.alloc_impl(layout);

        println!("AFTER ALLOC (received {:p})", ptr);
        self.dump();

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        println!("BEFORE DEALLOC: {:p}", ptr);
        self.dump();
        self.dealloc_impl(ptr, layout);
        println!("AFTER DEALLOC");
        self.dump();
    }
}
