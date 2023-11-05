use core::{alloc::GlobalAlloc, cell::RefCell, cmp::Ordering, ptr::NonNull};

use common::mutex::{Mutex, MutexGuard};

use crate::{debug, info, klibc::util::align_up};

use super::page_allocator;

const DELIMITER: &str = "######################################################";

struct HeapInner {
    free_list: Option<NonNull<FreeBlock>>,
}

struct Heap {
    inner: RefCell<HeapInner>,
}

#[repr(packed)]
struct FreeBlock {
    next: Option<NonNull<FreeBlock>>,
    size: usize,
}

impl FreeBlock {
    fn get_data_ptr(&self) -> *mut u8 {
        let free_block_ptr = self as *const FreeBlock as *const u8 as *mut u8;
        unsafe { free_block_ptr.byte_add(core::mem::size_of::<FreeBlock>()) }
    }
}

struct MutexHeap {
    inner: Mutex<Heap>,
}

impl MutexHeap {
    const fn new() -> Self {
        Self {
            inner: Mutex::new(Heap::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Heap> {
        self.inner.lock()
    }
}

#[global_allocator]
static OS_HEAP: MutexHeap = MutexHeap::new();

impl Heap {
    const fn new() -> Self {
        Self {
            inner: RefCell::new(HeapInner { free_list: None }),
        }
    }

    fn init(&mut self, start: *mut u8, size: usize) {
        let align_start = align_to(start as usize);
        let difference = align_start - start as usize;
        let size = size - difference;

        let free_block: &mut FreeBlock = unsafe { &mut *(align_start as *mut FreeBlock) };

        free_block.next = None;
        free_block.size = size;

        self.inner.borrow_mut().free_list = NonNull::new(free_block);
    }

    fn dump(&self) {
        debug!("{}", DELIMITER);
        debug!("Heap DUMP of free blocks");
        debug!("START\t\t\tEND\t\t\tSIZE");

        let mut free_block_holder = self.inner.borrow().free_list;

        unsafe {
            while let Some(free_block) = free_block_holder {
                let free_block_addr = free_block.as_ptr();
                let free_block = free_block.as_ref();
                let size = free_block.size;

                debug!(
                    "{:p}\t\t{:p}\t\t0x{:x}",
                    free_block_addr,
                    (free_block_addr.byte_add(size)),
                    size
                );

                free_block_holder = free_block.next;
            }
        }
        debug!("{}", DELIMITER);
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
                free_block_ref.size = size;

                let new_free_block = &mut *free_block.as_ptr().byte_add(size);
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
    OS_HEAP.lock().dump();
}

pub fn init() {
    let heap_start = page_allocator::zalloc(1).unwrap();

    OS_HEAP
        .lock()
        .init(heap_start.addr().cast().as_ptr(), page_allocator::PAGE_SIZE);
    info!(
        "Heap initialized! (Start: 0x{:p} Size: 0x{:x})\n",
        heap_start.addr().as_ptr(),
        page_allocator::PAGE_SIZE
    );
}

unsafe impl GlobalAlloc for MutexHeap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let heap = self.lock();
        let size = align_to(layout.size() + core::mem::size_of::<FreeBlock>());
        // debug!(
        //     "BEFORE ALLOC: 0x{:x} (Original: 0x{:x})",
        //     size,
        //     layout.size()
        // );
        // heap.dump();

        let mut ptr = heap.alloc_impl(layout);

        // If the heap is empty we try to allocate more from the page allocator
        if ptr.is_null() {
            debug!("Try to allocate from page_allocator");

            let number_of_pages =
                align_up(size, page_allocator::PAGE_SIZE) / page_allocator::PAGE_SIZE;
            let pages = match page_allocator::zalloc(number_of_pages) {
                None => return ptr,
                Some(pages) => pages,
            };
            let free_block: &mut FreeBlock = pages.addr().cast().as_mut();
            free_block.next = None;
            free_block.size = number_of_pages * page_allocator::PAGE_SIZE;
            ptr = free_block.get_data_ptr();
        }

        // debug!("AFTER ALLOC (received {:p})", ptr);
        // heap.dump();

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let heap = self.lock();
        // debug!("BEFORE DEALLOC: {:p}", ptr);
        // heap.dump();
        heap.dealloc_impl(ptr, layout);
        // debug!("AFTER DEALLOC");
        // heap.dump();
    }
}
