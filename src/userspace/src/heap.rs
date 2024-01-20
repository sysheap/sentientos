/**
 * This is a quick&dirty copy&paste from the kernel to supply a heap for the userspace.
 * Either share the code or implement it properly!
 */
use core::{
    alloc::{GlobalAlloc, Layout},
    marker::PhantomData,
    mem::{align_of, size_of},
    ops::{Deref, DerefMut, Range},
    ptr::{null_mut, NonNull},
};

use common::{mutex::Mutex, syscalls};

const PAGE_SIZE: usize = 4096;

const fn minimum_amount_of_pages(value: usize) -> usize {
    align_up(value, PAGE_SIZE) / PAGE_SIZE
}

const fn align_up(value: usize, alignment: usize) -> usize {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(C, align(4096))]
pub struct Page([u8; PAGE_SIZE]);

impl Deref for Page {
    type Target = [u8; PAGE_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Page {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Page {
    fn zero() -> Self {
        Self([0; PAGE_SIZE])
    }
}

trait Pages {
    fn as_u8_slice(&mut self) -> &mut [u8];
}

impl Pages for [Page] {
    fn as_u8_slice(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.as_mut_ptr() as *mut u8,
                core::mem::size_of_val(self),
            )
        }
    }
}

pub trait PageAllocator {
    fn alloc(number_of_pages_requested: usize) -> Option<Range<NonNull<Page>>>;
    fn dealloc(page: NonNull<Page>);
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
struct AlignedSizeWithMetadata {
    size: usize,
}

impl AlignedSizeWithMetadata {
    const fn new() -> Self {
        Self { size: 0 }
    }

    fn from_layout(layout: Layout) -> Self {
        assert!(FreeBlock::DATA_ALIGNMENT >= layout.align());
        let size = align_up(
            core::cmp::max(layout.size(), FreeBlock::MINIMUM_SIZE),
            FreeBlock::DATA_ALIGNMENT,
        );
        Self { size }
    }

    const fn from_pages(pages: usize) -> Self {
        Self {
            size: align_up(pages * PAGE_SIZE, FreeBlock::DATA_ALIGNMENT),
        }
    }

    const fn total_size(&self) -> usize {
        self.size
    }

    const fn get_remaining_size(&self, needed_size: AlignedSizeWithMetadata) -> Self {
        assert!(self.total_size() >= needed_size.total_size() + FreeBlock::MINIMUM_SIZE);
        Self {
            size: self.size - needed_size.size,
        }
    }
}

#[repr(C, align(8))]
struct FreeBlock {
    next: Option<NonNull<FreeBlock>>,
    size: AlignedSizeWithMetadata,
    // data: u64, This field is virtual because otherwise the offset calculation would be wrong
}

impl FreeBlock {
    const METADATA_SIZE: usize = size_of::<Self>();
    const DATA_ALIGNMENT: usize = align_of::<usize>();
    const MINIMUM_SIZE: usize = Self::METADATA_SIZE + Self::DATA_ALIGNMENT;

    const fn new() -> Self {
        Self {
            next: None,
            size: AlignedSizeWithMetadata::new(),
        }
    }

    const fn new_with_size(size: AlignedSizeWithMetadata) -> Self {
        Self { next: None, size }
    }

    fn initialize(block_ptr: NonNull<FreeBlock>, size: AlignedSizeWithMetadata) {
        let data_size = size.total_size();

        assert!(data_size >= Self::MINIMUM_SIZE);

        assert!(data_size >= Self::DATA_ALIGNMENT, "FreeBlock too small");
        assert!(
            data_size % Self::DATA_ALIGNMENT == 0,
            "FreeBlock not aligned (data_size={data_size})"
        );

        let block = FreeBlock::new_with_size(size);
        unsafe {
            block_ptr.write(block);
        }
    }

    fn split(
        mut block_ptr: NonNull<FreeBlock>,
        requested_size: AlignedSizeWithMetadata,
    ) -> NonNull<FreeBlock> {
        let block = unsafe { block_ptr.as_mut() };
        assert!(block.size.total_size() >= requested_size.total_size() + Self::MINIMUM_SIZE);
        assert!(requested_size.total_size() % Self::DATA_ALIGNMENT == 0);

        let remaining_size = block.size.get_remaining_size(requested_size);

        let new_block = unsafe { block_ptr.byte_add(requested_size.total_size()) };

        assert!(remaining_size.total_size() % Self::DATA_ALIGNMENT == 0);

        block.size = requested_size;

        Self::initialize(new_block, remaining_size);
        new_block
    }
}

struct Heap<Allocator: PageAllocator> {
    genesis_block: FreeBlock,
    allocator: PhantomData<Allocator>,
}

impl<Allocator: PageAllocator> Heap<Allocator> {
    const fn new() -> Self {
        Self {
            genesis_block: FreeBlock::new(),
            allocator: PhantomData,
        }
    }

    fn is_page_allocator_allocation(&self, layout: &Layout) -> bool {
        layout.size() >= PAGE_SIZE || layout.align() == PAGE_SIZE
    }

    fn alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        if self.is_page_allocator_allocation(&layout) {
            // Allocate directly from the page allocator
            let pages = minimum_amount_of_pages(layout.size());
            if let Some(allocation) = Allocator::alloc(pages) {
                return allocation.start.cast().as_ptr();
            } else {
                return null_mut();
            };
        }

        let requested_size = AlignedSizeWithMetadata::from_layout(layout);
        let block = if let Some(block) = self.find_and_remove(requested_size) {
            block
        } else {
            let pages = minimum_amount_of_pages(requested_size.total_size());
            let allocation = if let Some(allocation) = Allocator::alloc(pages) {
                allocation
            } else {
                return null_mut();
            };
            let free_block_ptr = allocation.start.cast();
            FreeBlock::initialize(free_block_ptr, AlignedSizeWithMetadata::from_pages(pages));
            free_block_ptr
        };

        // Make smaller if needed
        self.split_if_necessary(block, requested_size);

        block.cast().as_ptr()
    }

    fn dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout) {
        assert!(!ptr.is_null());
        if self.is_page_allocator_allocation(&layout) {
            // Deallocate directly to the page allocator
            unsafe {
                Allocator::dealloc(NonNull::new_unchecked(ptr).cast());
            }
            return;
        }
        let size = AlignedSizeWithMetadata::from_layout(layout);
        let free_block_ptr = unsafe { NonNull::new_unchecked(ptr).cast() };
        let free_block = FreeBlock::new_with_size(size);
        unsafe {
            free_block_ptr.write(free_block);
            self.insert(free_block_ptr);
        }
    }

    fn insert(&mut self, mut block_ptr: NonNull<FreeBlock>) {
        let block = unsafe { block_ptr.as_mut() };
        assert!(block.next.is_none(), "Heap metadata corruption");
        block.next = self.genesis_block.next.take();
        self.genesis_block.next = Some(block_ptr);
    }

    fn split_if_necessary(
        &mut self,
        block_ptr: NonNull<FreeBlock>,
        requested_size: AlignedSizeWithMetadata,
    ) {
        let block = unsafe { block_ptr.as_ref() };
        let current_block_size = block.size;
        assert!(current_block_size >= requested_size);
        if (current_block_size.total_size() - requested_size.total_size()) < FreeBlock::MINIMUM_SIZE
        {
            return;
        }
        let new_block = FreeBlock::split(block_ptr, requested_size);
        self.insert(new_block);
    }

    fn find_and_remove(
        &mut self,
        requested_size: AlignedSizeWithMetadata,
    ) -> Option<NonNull<FreeBlock>> {
        let mut current = &mut self.genesis_block;
        while let Some(potential_block) = current.next.map(|mut block| unsafe { block.as_mut() }) {
            if potential_block.size < requested_size {
                current = potential_block;
                continue;
            }

            // Take the block out of the list
            let block = current.next.take();
            current.next = potential_block.next.take();
            return block;
        }
        None
    }
}

struct MutexHeap<Allocator: PageAllocator> {
    inner: Mutex<Heap<Allocator>>,
}

impl<Allocator: PageAllocator> MutexHeap<Allocator> {
    const fn new() -> Self {
        Self {
            inner: Mutex::new(Heap::new()),
        }
    }
}

unsafe impl<Allocator: PageAllocator> GlobalAlloc for MutexHeap<Allocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.inner.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.inner.lock().dealloc(ptr, layout)
    }
}

struct KernelSyscallAllocator;

impl PageAllocator for KernelSyscallAllocator {
    fn alloc(number_of_pages_requested: usize) -> Option<Range<NonNull<Page>>> {
        let ptr = syscalls::userspace::MMAP_PAGES(number_of_pages_requested) as *mut Page;
        if ptr.is_null() {
            return None;
        }
        // SAFETY: We allready checked for a null ptr
        unsafe {
            let end = ptr.add(number_of_pages_requested);
            Some(NonNull::new_unchecked(ptr)..NonNull::new_unchecked(end))
        }
    }

    fn dealloc(page: NonNull<Page>) {
        todo!()
    }
}

#[global_allocator]
static HEAP: MutexHeap<KernelSyscallAllocator> = MutexHeap::new();
