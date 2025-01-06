#![no_std]
#![no_main]

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use common::syscalls::{
    sys_execute, sys_execute_add_arg, sys_execute_arg_clear, sys_exit, sys_print_programs, sys_wait,
};
use userspace::{print, println, util::read_line};

extern crate alloc;
extern crate userspace;

#[unsafe(no_mangle)]
fn main() {
    println!();
    println!("### SeSH - Sentient Shell ###");
    println!("Type 'help' for a list of available commands.");
    loop {
        print!("$ ");
        let input = read_line();
        // Parse input and execute
        parse_command_and_execute(input);
    }
}

fn parse_command_and_execute(mut command: String) {
    command = command.trim().to_string();
    match command.as_str() {
        "" => {}
        "exit" | "q" => {
            println!("Exiting...");
            sys_exit(0);
        }
        "help" => {
            println!("Available commands:");
            println!("exit - Exit the shell");
            println!("help - Print this help message");
            println!("\nFollowing programs exist and can be called:");
            sys_print_programs();
        }
        _ => {
            let mut background = false;

            if command.ends_with('&') {
                background = true;
                command.pop();
                command = command.trim().to_string();
            }

            // Process arguments
            let split: Vec<&str> = command.split(' ').collect();

            sys_execute_arg_clear();

            for arg in split.iter().skip(1).filter(|arg| !arg.trim().is_empty()) {
                sys_execute_add_arg(arg).expect("Succeed");
            }

            let execute_result = sys_execute(split[0]);
            match execute_result {
                Ok(pid) => {
                    if !background {
                        let _ = sys_wait(pid);
                    }
                }
                Err(err) => {
                    println!("Error executing program: {:?}", err);
                }
            }
        }
    }
}
