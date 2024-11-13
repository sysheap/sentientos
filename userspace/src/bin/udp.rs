#![no_std]
#![no_main]

use alloc::string::String;
use common::syscalls::sys_read_input;
use userspace::{net::UdpSocket, print, println};

extern crate alloc;
extern crate userspace;

const PORT: u16 = 1234;
const DELETE: u8 = 127;

#[no_mangle]
fn main() {
    println!("Hello from the udp receiver");
    println!("Listening on {PORT}");

    let mut socket = UdpSocket::try_open(PORT).expect("Socket must be openable.");
    let mut input = String::new();

    loop {
        let mut buffer = [0; 64];
        let count = socket.receive(&mut buffer);

        if count > 0 {
            let text = core::str::from_utf8(&buffer[0..count]).expect("Must be valid utf8");
            print!("{}", text);
        }

        if let Some(c) = sys_read_input() {
            match c {
                b'\r' => {
                    // Carriage return
                    // Send data
                    println!();
                    input.push(b'\n' as char);
                    socket.transmit(input.as_bytes());
                    input.clear();
                }
                DELETE => {
                    if input.pop().is_some() {
                        print!("{}{}{}", 8 as char, ' ', 8 as char);
                    }
                }
                _ => {
                    assert!(c.is_ascii());
                    let result = c as char;
                    input.push(result);
                    print!("{}", result);
                }
            }
        }
    }
}
