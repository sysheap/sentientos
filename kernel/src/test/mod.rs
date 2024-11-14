use crate::{print, println};

mod array_vec;
mod leb128;
mod mutex;

pub mod qemu_exit;

// Inspired by https://os.phil-opp.com/testing/

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("TEST: {}\n", core::any::type_name::<T>());
        self();
    }
}

#[allow(dead_code)]
pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    crate::memory::initialize_runtime_mappings(&[]);
    // #[cfg(miri)]
    // {
    //     use crate::memory::{self, PAGE_SIZE};
    //     use core::alloc::Layout;

    //     // Allocate memory for the page allocator
    //     let heap_size = 16 * 1024 * 1024; // 128 MB should be enough for the tests
    //     let page_allocator_layout = Layout::from_size_align(heap_size, PAGE_SIZE).unwrap();
    //     let page_allocator_heap = unsafe { std::alloc::alloc_zeroed(page_allocator_layout) };
    //     assert!(!page_allocator_heap.is_null());
    //     memory::init_page_allocator(page_allocator_heap, heap_size);
    // }
    for test in tests {
        test.run();
    }
    #[cfg(not(miri))]
    qemu_exit::exit_success();
}
