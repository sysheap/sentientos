use core::{marker::PhantomData, ptr::NonNull, slice};

use crate::{debug, memory::PAGE_ALLOCATOR};

use super::page_allocator::{Page, PAGE_SIZE};

#[derive(Debug, Default)]
pub struct Ephemeral;
#[derive(Debug, Default)]
pub struct Ethernal;

pub trait PageDropper: Sized {
    fn drop<A: WhichAllocator>(page: NonNull<Page>);
}

impl PageDropper for Ephemeral {
    fn drop<A: WhichAllocator>(pages: NonNull<Page>) {
        debug!("Drop allocated page at {:p}", pages);
        A::deallocate(pages);
    }
}
impl PageDropper for Ethernal {
    fn drop<A: WhichAllocator>(_page: NonNull<Page>) {}
}

pub trait WhichAllocator {
    fn allocate(number_of_pages: usize) -> Option<NonNull<Page>>;
    fn deallocate(pages: NonNull<Page>);
}

#[derive(Debug)]
pub struct StaticAllocator;

impl WhichAllocator for StaticAllocator {
    fn allocate(number_of_pages: usize) -> Option<NonNull<Page>> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages)
    }

    fn deallocate(pages: NonNull<Page>) {
        PAGE_ALLOCATOR.lock().dealloc(pages);
    }
}

#[derive(Debug)]
pub struct AllocatedPages<Dropper: PageDropper, A: WhichAllocator = StaticAllocator> {
    ptr: NonNull<Page>,
    number_of_pages: usize,
    dropper_phantom: PhantomData<Dropper>,
    which_allocator_phantom: PhantomData<A>,
}

impl<Dropper: PageDropper, A: WhichAllocator> AllocatedPages<Dropper, A> {
    pub fn zalloc(number_of_pages: usize) -> Option<Self> {
        A::allocate(number_of_pages).map(|ptr| {
            let mut allocated_page = Self::new(ptr, number_of_pages);
            allocated_page.zero();
            allocated_page
        })
    }

    fn new(ptr: NonNull<Page>, number_of_pages: usize) -> Self {
        Self {
            ptr,
            number_of_pages,
            dropper_phantom: PhantomData,
            which_allocator_phantom: PhantomData,
        }
    }

    pub fn addr(&self) -> NonNull<Page> {
        self.ptr
    }

    fn u8(&self) -> *mut u8 {
        self.ptr.cast().as_ptr()
    }

    pub fn slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.u8(), self.number_of_pages * PAGE_SIZE) }
    }

    pub fn addr_as_usize(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    pub fn zero(&mut self) {
        for offset in 0..self.number_of_pages {
            unsafe {
                self.ptr.as_ptr().add(offset).as_mut().unwrap().fill(0);
            }
        }
    }
}

impl<Dropper: PageDropper, A: WhichAllocator> Drop for AllocatedPages<Dropper, A> {
    fn drop(&mut self) {
        Dropper::drop::<A>(self.ptr);
        self.number_of_pages = 0;
        self.ptr = NonNull::dangling();
    }
}
