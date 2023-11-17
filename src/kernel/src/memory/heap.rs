use core::{alloc::GlobalAlloc, cmp::Ordering, ptr::NonNull};

use common::mutex::{Mutex, MutexGuard};

use crate::{
    debug,
    klibc::util::align_up,
    memory::page_allocator::{AllocatedPages, Ethernal},
};

use super::page_allocator;

struct Heap {
    free_list: Option<NonNull<FreeBlock>>,
}

#[repr(packed)]
struct FreeBlock {
    next: Option<NonNull<FreeBlock>>,
    size: usize,
}

impl FreeBlock {
    fn get_data_ptr(&self) -> *mut u8 {
        let free_block_ptr = self as *const FreeBlock;
        unsafe { free_block_ptr.add(1) as *mut u8 }
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
        Self { free_list: None }
    }

    fn dump(&self) {
        debug!("Heap DUMP of free blocks");
        debug!("START\t\t\tEND\t\t\tSIZE");

        let mut free_block_holder = self.free_list;

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
    }

    unsafe fn alloc_impl(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        let mut size = align_to(layout.size() + core::mem::size_of::<FreeBlock>());

        let mut previous_free_block = None;

        let mut free_block_option = self.free_list;

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
                    None => self.free_list = free_block_ref.next,
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
                    None => self.free_list = NonNull::new(new_free_block),
                    Some(mut previous_free_block) => {
                        previous_free_block.as_mut().next = NonNull::new(new_free_block)
                    }
                }

                free_block_ref.get_data_ptr()
            }
            Ordering::Greater => panic!(""),
        };
    }

    unsafe fn dealloc_impl(&mut self, ptr: *mut u8, _layout: core::alloc::Layout) {
        let freeblock = &mut *(ptr.sub(core::mem::size_of::<FreeBlock>()) as *mut FreeBlock);

        freeblock.next = self.free_list;

        self.free_list = NonNull::new(freeblock);
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

#[allow(dead_code)]
pub fn dump() {
    OS_HEAP.lock().dump();
}

unsafe impl GlobalAlloc for MutexHeap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut heap = self.lock();
        let size = align_to(layout.size() + core::mem::size_of::<FreeBlock>());

        let mut ptr = heap.alloc_impl(layout);

        // If the heap is empty we try to allocate more from the page allocator
        if ptr.is_null() {
            debug!("Try to allocate from page_allocator");

            let number_of_pages =
                align_up(size, page_allocator::PAGE_SIZE) / page_allocator::PAGE_SIZE;
            let pages = match AllocatedPages::<Ethernal>::zalloc(number_of_pages) {
                None => return ptr,
                Some(pages) => pages,
            };
            let free_block: &mut FreeBlock = pages.addr().cast().as_mut();
            free_block.next = None;
            free_block.size = number_of_pages * page_allocator::PAGE_SIZE;
            ptr = free_block.get_data_ptr();
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut heap = self.lock();
        heap.dealloc_impl(ptr, layout);
    }
}
