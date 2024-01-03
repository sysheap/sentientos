#![cfg_attr(miri, allow(unused_imports))]
use core::{
    alloc::{GlobalAlloc, Layout},
    marker::PhantomData,
    mem::{align_of, size_of},
    ptr::{null_mut, NonNull},
};

use common::mutex::Mutex;

use crate::{
    assert::static_assert_size,
    klibc::util::{align_up, minimum_amount_of_pages},
};

use super::{
    allocated_pages::{AllocatedPages, Ethernal, StaticAllocator, WhichAllocator},
    PAGE_SIZE,
};

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

static_assert_size!(FreeBlock, 16);

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

struct Heap<A: WhichAllocator> {
    genesis_block: FreeBlock,
    allocator: PhantomData<A>,
}

impl<A: WhichAllocator> Heap<A> {
    const fn new() -> Self {
        Self {
            genesis_block: FreeBlock::new(),
            allocator: PhantomData,
        }
    }

    fn alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        let requested_size = AlignedSizeWithMetadata::from_layout(layout);
        let block = if let Some(block) = self.find_and_remove(requested_size) {
            block
        } else {
            let pages = minimum_amount_of_pages(requested_size.total_size());
            let allocation = if let Some(allocation) = AllocatedPages::<Ethernal, A>::zalloc(pages)
            {
                allocation
            } else {
                return null_mut();
            };
            let free_block_ptr = allocation.addr().cast();
            FreeBlock::initialize(free_block_ptr, AlignedSizeWithMetadata::from_pages(pages));
            free_block_ptr
        };

        // Make smaller if needed
        self.split_if_necessary(block, requested_size);

        block.cast().as_ptr()
    }

    fn dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout) {
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
        let mut previous_block = &mut self.genesis_block;
        unsafe {
            loop {
                let block = previous_block
                    .next
                    .take_if(|block| block.as_ref().size >= requested_size)
                    .map(|mut block| {
                        previous_block.next = block.as_mut().next.take();
                        block
                    });
                if block.is_some() {
                    return block;
                }
                if let Some(next) = &mut previous_block.next {
                    previous_block = next.as_mut();
                } else {
                    break;
                }
            }
        }
        None
    }
}

struct MutexHeap<A: WhichAllocator> {
    inner: Mutex<Heap<A>>,
}

impl<A: WhichAllocator> MutexHeap<A> {
    const fn new() -> Self {
        Self {
            inner: Mutex::new(Heap::new()),
        }
    }
}

unsafe impl<A: WhichAllocator> GlobalAlloc for MutexHeap<A> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.inner.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.inner.lock().dealloc(ptr, layout)
    }
}

#[cfg(not(miri))]
#[global_allocator]
static HEAP: MutexHeap<StaticAllocator> = MutexHeap::new();

#[cfg(test)]
mod test {
    use core::alloc::GlobalAlloc;

    use common::mutex::Mutex;

    use crate::memory::{
        allocated_pages::WhichAllocator,
        page_allocator::{Page, PageAllocator},
    };

    use super::{FreeBlock, MutexHeap, PAGE_SIZE};

    const HEAP_PAGES: usize = 8;

    static mut PAGE_ALLOC_MEMORY: [u8; PAGE_SIZE * HEAP_PAGES] = [0; PAGE_SIZE * HEAP_PAGES];
    static PAGE_ALLOC: Mutex<PageAllocator> = Mutex::new(PageAllocator::new());

    struct TestAllocator;
    impl WhichAllocator for TestAllocator {
        fn allocate(number_of_pages: usize) -> Option<core::ptr::NonNull<Page>> {
            PAGE_ALLOC.lock().alloc(number_of_pages)
        }

        fn deallocate(pages: core::ptr::NonNull<Page>) {
            PAGE_ALLOC.lock().dealloc(pages)
        }
    }

    fn init_allocator() {
        unsafe {
            PAGE_ALLOC.lock().init(&mut PAGE_ALLOC_MEMORY);
        }
    }

    fn create_heap() -> MutexHeap<TestAllocator> {
        init_allocator();
        MutexHeap::<TestAllocator>::new()
    }

    fn alloc<T>(heap: &MutexHeap<TestAllocator>) -> *mut T {
        let layout = core::alloc::Layout::new::<T>();
        let ptr = unsafe { heap.alloc(layout) as *mut T };
        ptr
    }

    fn dealloc<T>(heap: &MutexHeap<TestAllocator>, ptr: *mut T) {
        let layout = core::alloc::Layout::new::<T>();
        unsafe { heap.dealloc(ptr as *mut u8, layout) };
    }

    #[test_case]
    fn empty_heap() {
        let heap = create_heap();
        assert!(heap.inner.lock().genesis_block.next.is_none());
    }

    #[test_case]
    fn single_allocation() {
        let heap = create_heap();
        let ptr = alloc::<u8>(&heap);
        assert!(!ptr.is_null());
        unsafe {
            ptr.write(0x42);
        };
        let heap = heap.inner.lock();
        let free_block = unsafe { heap.genesis_block.next.unwrap().as_ref() };
        assert!(free_block.next.is_none());
        assert_eq!(
            free_block.size.total_size(),
            PAGE_SIZE - FreeBlock::METADATA_SIZE - FreeBlock::DATA_ALIGNMENT
        );
    }

    #[test_case]
    fn split_block() {
        let heap = create_heap();
        let ptr1 = alloc::<u8>(&heap);
        assert!(!ptr1.is_null());
        unsafe {
            ptr1.write(0x42);
        };

        let ptr2 = alloc::<u8>(&heap);
        assert!(!ptr2.is_null());
        unsafe {
            ptr2.write(0x42);
        };

        let heap = heap.inner.lock();
        let free_block = unsafe { heap.genesis_block.next.unwrap().as_ref() };
        assert!(free_block.next.is_none());
        assert_eq!(
            free_block.size.total_size(),
            PAGE_SIZE - (2 * FreeBlock::METADATA_SIZE) - (2 * FreeBlock::DATA_ALIGNMENT)
        );
    }

    #[test_case]
    fn deallocation() {
        let heap = create_heap();
        let ptr = alloc::<u8>(&heap);
        assert!(!ptr.is_null());
        unsafe {
            ptr.write(0x42);
        };

        dealloc(&heap, ptr);
        let heap = heap.inner.lock();
        let free_block1 = unsafe { heap.genesis_block.next.unwrap().as_ref() };
        assert_eq!(free_block1.size.total_size(), FreeBlock::MINIMUM_SIZE);

        let free_block2 = unsafe { free_block1.next.unwrap().as_ref() };
        assert!(free_block2.next.is_none());
        assert_eq!(
            free_block2.size.total_size(),
            PAGE_SIZE - FreeBlock::METADATA_SIZE - FreeBlock::DATA_ALIGNMENT
        );
    }

    #[test_case]
    fn alloc_exhaustion() {
        let heap = create_heap();
        // One page is metadata
        const SIZE: usize = (HEAP_PAGES - 1) * PAGE_SIZE;
        let ptr = alloc::<[u8; SIZE]>(&heap);
        assert!(!ptr.is_null());
        unsafe {
            ptr.write([0x42; SIZE]);
        };

        let ptr2 = alloc::<u8>(&heap);
        assert!(ptr2.is_null());

        let heap_lock = heap.inner.lock();
        assert!(heap_lock.genesis_block.next.is_none());
        drop(heap_lock);

        dealloc(&heap, ptr);

        let ptr = alloc::<u8>(&heap);
        assert!(!ptr.is_null());
        unsafe {
            ptr.write(0x42);
        }

        let heap_lock = heap.inner.lock();
        let free_block = unsafe { heap_lock.genesis_block.next.unwrap().as_ref() };
        assert!(free_block.next.is_none());
        assert_eq!(free_block.size.total_size(), SIZE - FreeBlock::MINIMUM_SIZE);
    }
}
