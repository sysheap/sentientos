use common::ioctl::print_programs;
use std::{
    io::{Write, stdout},
    string::{String, ToString},
    vec::Vec,
};
use userspace::{spawn::spawn, util::read_line};

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

            let mut split = command.split(' ');
            let prog_name = split.next().unwrap_or(&command);
            let args: Vec<&str> = split.filter(|arg| !arg.trim().is_empty()).collect();

            match spawn(prog_name, &args) {
                Ok(pid) => {
                    if !background {
                        unsafe {
                            libc::waitpid(pid, core::ptr::null_mut(), 0);
                        }
                    }
                }
                Err(err) => {
                    println!("Error executing program: {err}");
                }
            }
        }
    }
}
