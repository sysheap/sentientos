use crate::{heap, page_allocator, page_tables, plic, println, uart};

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

#[no_mangle]
extern "C" fn kernel_init() {
    uart::QEMU_UART.init();
    println!("Hello World from YaROS!\n");

    unsafe {
        println!("Initializing page allocator");
        page_allocator::init(HEAP_START, HEAP_SIZE);
        heap::init();
    }

    page_tables::setup_kernel_identity_mapping();

    println!("kernel_init() completed!");
}

#[no_mangle]
extern "C" fn kernel_main() {
    println!("kernel_main()");

    plic::init_uart_interrupt();
}
