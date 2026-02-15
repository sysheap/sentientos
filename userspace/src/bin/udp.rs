use std::io::{Write, stdout};

use userspace::net::UdpSocket;

extern crate alloc;
extern crate userspace;

const PORT: u16 = 1234;
const DELETE: u8 = 127;

fn main() {
    println!("Hello from the udp receiver");
    println!("Listening on {PORT}");

    unsafe { libc::fcntl(0, libc::F_SETFL, libc::O_NONBLOCK) };

    let mut socket = UdpSocket::try_open(PORT).expect("Socket must be openable.");
    let mut input = String::new();

    loop {
        let mut buffer = [0; 64];
        let count = socket.receive(&mut buffer);

        if count > 0 {
            let text = std::str::from_utf8(&buffer[0..count]).expect("Must be valid utf8");
            print!("{}", text);
            let _ = stdout().flush();
        }

        let mut c = 0u8;
        let ret = unsafe { libc::read(0, &mut c as *mut u8 as *mut libc::c_void, 1) };
        if ret == 1 {
            match c {
                b'\r' | b'\n' => {
                    println!();
                    input.push(b'\n' as char);
                    socket.transmit(input.as_bytes());
                    input.clear();
                }
                DELETE => {
                    if input.pop().is_some() {
                        print!("{} {}", 8 as char, 8 as char);
                        let _ = stdout().flush();
                    }
                }
                _ => {
                    assert!(c.is_ascii());
                    let result = c as char;
                    input.push(result);
                    print!("{}", result);
                    let _ = stdout().flush();
                }
            }
        }
    }
}
