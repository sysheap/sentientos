use core::{marker::PhantomData, ptr::NonNull, slice};

use crate::{debug, memory::PAGE_ALLOCATOR};

use super::page_allocator::{Page, PAGE_SIZE};

#[derive(Debug, Default)]
pub struct Ephemeral;
#[derive(Debug, Default)]
pub struct Ethernal;

pub trait PageDropper: Sized {
    fn drop(page: &mut AllocatedPages<Self>);
}

impl PageDropper for Ephemeral {
    fn drop(page: &mut AllocatedPages<Self>) {
        debug!("Drop allocated page at {:p}", page.ptr.as_ptr());
        PAGE_ALLOCATOR.lock().dealloc(page.ptr);
        page.number_of_pages = 0;
        page.ptr = NonNull::dangling();
    }
}
impl PageDropper for Ethernal {
    fn drop(_page: &mut AllocatedPages<Self>) {}
}

#[derive(Debug)]
pub struct AllocatedPages<Dropper: PageDropper> {
    ptr: NonNull<Page>,
    number_of_pages: usize,
    phantom: PhantomData<Dropper>,
}

impl<Dropper: PageDropper> AllocatedPages<Dropper> {
    pub fn zalloc(number_of_pages: usize) -> Option<Self> {
        PAGE_ALLOCATOR.lock().alloc(number_of_pages).map(|ptr| {
            let mut allocated_page = Self::new(ptr, number_of_pages);
            allocated_page.zero();
            allocated_page
        })
    }

    fn new(ptr: NonNull<Page>, number_of_pages: usize) -> Self {
        Self {
            ptr,
            number_of_pages,
            phantom: PhantomData,
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

impl<Dropper: PageDropper> Drop for AllocatedPages<Dropper> {
    fn drop(&mut self) {
        Dropper::drop(self);
    }
}
