use core::{alloc::GlobalAlloc, cell::RefCell, cmp::Ordering, ptr::NonNull};

use crate::{page_allocator, println, util::align_up};

const DELIMITER: &str = "######################################################";

struct HeapInner {
    free_list: Option<NonNull<FreeBlock>>,
}

struct Heap {
    inner: RefCell<HeapInner>,
    start_addr: *const u8,
    size: usize,
}

#[repr(packed)]
struct FreeBlock {
    next: Option<NonNull<FreeBlock>>,
    size: usize,
    used: usize,
}

impl FreeBlock {
    fn get_data_ptr(&self) -> *mut u8 {
        let free_block_ptr = self as *const FreeBlock as *const u8 as *mut u8;
        unsafe { free_block_ptr.byte_add(core::mem::size_of::<FreeBlock>()) }
    }
}

#[global_allocator]
static mut OS_HEAP: Heap = Heap::new();

impl Heap {
    const fn new() -> Self {
        Self {
            inner: RefCell::new(HeapInner { free_list: None }),
            start_addr: core::ptr::null(),
            size: 0,
        }
    }

    fn init(&mut self, start: *mut u8, size: usize) {
        let align_start = align_to(start as usize);
        let difference = align_start - start as usize;
        let size = size - difference;

        let free_block: &mut FreeBlock = unsafe { &mut *(align_start as *mut FreeBlock) };

        free_block.next = None;
        free_block.size = size;
        free_block.used = 0;

        self.inner.borrow_mut().free_list = NonNull::new(free_block);
        self.start_addr = align_start as *const u8;
        self.size = size;
    }

    fn dump(&self) {
        println!("{}", DELIMITER);
        println!("Heap DUMP");
        println!("USED\t\tSTART\t\tEND\t\tSIZE");

        let mut current_block = self.start_addr as *const FreeBlock;

        unsafe {
            while (current_block as usize) < (self.start_addr as usize + self.size) {
                let start_addr = current_block;
                let size = (*current_block).size;
                let used = if (*current_block).used == 0 {
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

                current_block = current_block.byte_add(size);
            }
        }
        println!("{}", DELIMITER);
    }

    unsafe fn alloc_impl(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut size = align_to(layout.size() + core::mem::size_of::<FreeBlock>());

        let mut previous_free_block = None;

        let mut free_block_option = self.inner.borrow_mut().free_list;

        // Find free block which is large enough
        unsafe {
            while let Some(free_block) = free_block_option {
                let free_block_ref = free_block.as_ref();
                if free_block_ref.size >= size {
                    break;
                }
                previous_free_block = Some(free_block);
                free_block_option = free_block_ref.next;
            }
        }

        let mut free_block = match free_block_option {
            None => return core::ptr::null_mut(),
            Some(x) => x,
        };

        let free_block_ref = free_block.as_mut();

        // Check if the rest of the block is too small to fit and consume it complete
        let remaining_size = free_block_ref.size - size;

        if remaining_size < core::mem::size_of::<FreeBlock>() {
            size = free_block_ref.size;
        }

        // Two cases: Hand out block completely or partially reduce it
        let free_block_size = free_block_ref.size;
        return match size.cmp(&free_block_size) {
            Ordering::Equal => {
                free_block_ref.used = 1;

                match previous_free_block {
                    None => self.inner.borrow_mut().free_list = free_block_ref.next,
                    Some(mut previous_free_block) => {
                        previous_free_block.as_mut().next = free_block_ref.next
                    }
                }

                free_block_ref.next = None;

                free_block_ref.get_data_ptr()
            }
            Ordering::Less => {
                let new_block_size = free_block_ref.size - size;
                free_block_ref.used = 1;
                free_block_ref.size = size;

                let new_free_block = &mut *free_block.as_ptr().byte_add(size);
                new_free_block.used = 0;
                new_free_block.size = new_block_size;
                new_free_block.next = free_block_ref.next;

                free_block_ref.next = None;

                match previous_free_block {
                    None => self.inner.borrow_mut().free_list = NonNull::new(new_free_block),
                    Some(mut previous_free_block) => {
                        previous_free_block.as_mut().next = NonNull::new(new_free_block)
                    }
                }

                free_block_ref.get_data_ptr()
            }
            Ordering::Greater => panic!(""),
        };
    }

    unsafe fn dealloc_impl(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let freeblock = &mut *(ptr.sub(core::mem::size_of::<FreeBlock>()) as *mut FreeBlock);

        freeblock.next = self.inner.borrow().free_list;
        freeblock.used = 0;

        self.inner.borrow_mut().free_list = NonNull::new(freeblock);
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
    let heap_start = page_allocator::zalloc(1).unwrap();
    unsafe {
        OS_HEAP.init(heap_start.addr().cast().as_ptr(), page_allocator::PAGE_SIZE);
        println!(
            "Heap initialized! (Start: 0x{:p} Size: 0x{:x})\n",
            heap_start.addr().as_ptr(),
            page_allocator::PAGE_SIZE
        );
    }
}

// TODO: This is unsafe! Our heap is not thread safe yet.
unsafe impl Sync for Heap {}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let size = align_to(layout.size() + core::mem::size_of::<FreeBlock>());
        println!(
            "BEFORE ALLOC: 0x{:x} (Original: 0x{:x})",
            size,
            layout.size()
        );
        self.dump();

        let mut ptr = self.alloc_impl(layout);

        // If the heap is empty we try to allocate more from the page allocator
        if ptr.is_null() {
            println!("Try to allocate from page_allocator");

            let number_of_pages =
                align_up(size, page_allocator::PAGE_SIZE) / page_allocator::PAGE_SIZE;
            let pages = match page_allocator::zalloc(number_of_pages) {
                None => return ptr,
                Some(pages) => pages,
            };
            let free_block: &mut FreeBlock = pages.addr().cast().as_mut();
            free_block.next = None;
            free_block.size = number_of_pages * page_allocator::PAGE_SIZE;
            free_block.used = 1;
            ptr = free_block.get_data_ptr();
        }

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
