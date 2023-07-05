use crate::println;

pub fn test_runner(tests: &[&dyn Fn()]) -> ! {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    #[allow(clippy::empty_loop)]
    loop {}
}
