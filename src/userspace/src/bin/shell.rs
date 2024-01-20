#![no_std]
#![no_main]

use alloc::string::String;
use common::syscalls::SYSCALL_WAIT;
use userspace::{print, println, util::wait};

extern crate alloc;
extern crate userspace;

const DELETE: u8 = 127;

#[no_mangle]
fn main() {
    println!();
    println!("### YaSH - Yet another Shell ###");
    println!("Type 'help' for a list of available commands.");
    loop {
        print!("$ ");
        let mut input = String::new();
        loop {
            let mut result: isize;
            loop {
                result = common::syscalls::userspace::READ_CHAR();
                if result != SYSCALL_WAIT {
                    break;
                }
                wait(10000);
            }
            let next_char = result as u8;
            match next_char {
                b'\r' => {
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
                    input.push(next_char as char);
                    print!("{}", next_char as char);
                }
            }
        }
        // Parse input and execute
        parse_command_and_execute(input);
    }
}

fn parse_command_and_execute(command: String) {
    match command.as_str() {
        "" => {}
        "exit" => {
            println!("Exiting...");
            common::syscalls::userspace::EXIT(0);
        }
        "help" => {
            println!("Available commands:");
            println!("exit - Exit the shell");
            println!("help - Print this help message");
        }
        program => {
            let reference = unsafe { &*program.as_ptr() };
            let mut len = program.len();

            if program.ends_with('&') {
                len -= 1;
            }

            let pid = common::syscalls::userspace::EXECUTE(reference, len);
            if pid < 0 {
                println!("Error executing program: {}", pid);
            } else if !program.ends_with('&') {
                common::syscalls::userspace::WAIT(pid as u64);
            }
        }
    }
}
