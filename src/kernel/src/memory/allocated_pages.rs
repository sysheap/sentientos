use core::{marker::PhantomData, ops::Range, ptr::NonNull, slice};

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
    fn allocate(number_of_pages: usize) -> Option<Range<NonNull<Page>>>;
    fn deallocate(pages: NonNull<Page>);
}

#[derive(Debug)]
pub struct StaticAllocator;

impl WhichAllocator for StaticAllocator {
    fn allocate(number_of_pages: usize) -> Option<Range<NonNull<Page>>> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages)
    }

    fn deallocate(pages: NonNull<Page>) {
        PAGE_ALLOCATOR.lock().dealloc(pages);
    }
}

#[derive(Debug)]
pub struct AllocatedPages<Dropper: PageDropper, A: WhichAllocator = StaticAllocator> {
    pages: Range<NonNull<Page>>,
    dropper_phantom: PhantomData<Dropper>,
    which_allocator_phantom: PhantomData<A>,
}

impl<Dropper: PageDropper, A: WhichAllocator> AllocatedPages<Dropper, A> {
    pub fn zalloc(number_of_pages: usize) -> Option<Self> {
        A::allocate(number_of_pages).map(|pages| {
            let mut allocated_page = Self::new(pages);
            allocated_page.zero();
            allocated_page
        })
    }

    fn new(pages: Range<NonNull<Page>>) -> Self {
        Self {
            pages,
            dropper_phantom: PhantomData,
            which_allocator_phantom: PhantomData,
        }
    }

    pub fn addr(&self) -> NonNull<Page> {
        self.pages.start
    }

    fn u8(&self) -> *mut u8 {
        self.addr().cast().as_ptr()
    }

    pub fn slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.u8(), self.number_of_pages() * PAGE_SIZE) }
    }

    pub fn page_slice(&mut self) -> &mut [Page] {
        unsafe { slice::from_raw_parts_mut(self.addr().as_ptr(), self.number_of_pages()) }
    }

    pub fn number_of_pages(&self) -> usize {
        unsafe { self.pages.end.offset_from(self.pages.start) as usize }
    }

    pub fn addr_as_usize(&self) -> usize {
        self.addr().as_ptr() as usize
    }

    pub fn zero(&mut self) {
        for page in self.page_slice() {
            page.fill(0);
        }
    }
}

impl<Dropper: PageDropper, A: WhichAllocator> Drop for AllocatedPages<Dropper, A> {
    fn drop(&mut self) {
        Dropper::drop::<A>(self.pages.start);
        self.pages = NonNull::dangling()..NonNull::dangling();
    }
}
