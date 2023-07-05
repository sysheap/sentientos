use crate::println;

mod qemu_exit;

pub use qemu_exit::TEST_DEVICE_ADDRESSS;

pub fn test_runner(tests: &[&dyn Fn()]) -> ! {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    qemu_exit::exit_success();
    #[allow(clippy::empty_loop)]
    loop {}
}
