extern crate alloc;

use alloc::string::String;
use common::syscalls::sys_read_input_wait;
use core::arch::asm;

use crate::{print, println};

const DELETE: u8 = 127;

pub fn wait(cycles: usize) {
    for _ in 0..cycles {
        unsafe {
            asm!("nop");
        }
    }
}

pub fn read_line() -> String {
    let mut input = String::new();
    loop {
        let result = sys_read_input_wait();
        match result {
            b'\r' | b'\n' => {
                // Carriage return
                println!();
                break;
            }
            DELETE => {
                if input.pop().is_some() {
                    print!("{}{}{}", 8 as char, ' ', 8 as char);
                }
            }
            _ => {
                assert!(result.is_ascii());
                let result = result as char;
                input.push(result);
                print!("{}", result);
            }
        }
    }
    input
}
