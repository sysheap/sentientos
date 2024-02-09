#![no_std]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod big_endian;
pub mod mutex;
pub mod numbers;
pub mod syscalls;

// Inspired by https://os.phil-opp.com/testing/

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        // print!("TEST: {}\n", core::any::type_name::<T>());
        self();
    }
}

#[allow(dead_code)]
fn test_runner(tests: &[&dyn Testable]) {
    // println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
}

#[cfg(test)]
pub fn mytest() {
    test_main();
}
