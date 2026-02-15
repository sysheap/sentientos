use common::ioctl::print_programs;
use common::syscalls::{sys_execute, sys_wait};
use std::{
    io::{Write, stdout},
    string::{String, ToString},
    vec::Vec,
};
use userspace::util::read_line;

extern crate alloc;
extern crate userspace;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("### SeSH - Sentient Shell ###");
    println!("Type 'help' for a list of available commands.");
    loop {
        print!("$ ");
        stdout().flush()?;
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
            std::process::exit(0);
        }
        "help" => {
            println!("Available commands:");
            println!("exit - Exit the shell");
            println!("help - Print this help message");
            println!("\nFollowing programs exist and can be called:");
            print_programs();
        }
        _ => {
            let mut background = false;

            if command.ends_with('&') {
                background = true;
                command.pop();
                command = command.trim().to_string();
            }

            // Process arguments
            let mut split = command.split(' ');

            let prog_name = split.next().unwrap_or(&command);

            let args: Vec<&str> = split.filter(|arg| !arg.trim().is_empty()).collect();

            let execute_result = sys_execute(prog_name, &args);
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
