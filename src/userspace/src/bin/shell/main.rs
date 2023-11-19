#![no_std]
#![no_main]

use common::{
    mutex::Mutex,
    syscalls::{SYSCALL_SUCCESS, SYSCALL_WAIT},
};
use userspace::{print, println, util::wait};

extern crate userspace;

static INPUT_BUFFER: Mutex<[u8; 1024]> = Mutex::new([0; 1024]);

#[no_mangle]
fn main() {
    println!();
    println!("### YaSH - Yet another Shell ###");
    println!("Type 'help' for a list of available commands.");
    loop {
        print!("$ ");
        let mut input_buffer = INPUT_BUFFER.lock();
        let mut buffer_index = 0;
        loop {
            let mut result: isize;
            loop {
                result = common::syscalls::userspace::READ_CHAR();
                if result != SYSCALL_WAIT {
                    break;
                }
                wait();
            }
            let next_char = result as u8;
            input_buffer[buffer_index] = next_char;
            buffer_index += 1;
            if next_char == b'\r' {
                // Carriage return
                println!();
                break;
            }
            print!("{}", next_char as char);
        }
        // Parse input and execute
        let command = core::str::from_utf8(&input_buffer[0..buffer_index - 1]).unwrap();
        parse_command_and_execute(command);
    }
}

fn parse_command_and_execute(command: &str) {
    match command {
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
