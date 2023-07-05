use crate::{print, println};

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
        print!("{}\t\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) -> ! {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    qemu_exit::exit_success();
    #[allow(clippy::empty_loop)]
    loop {}
}
