use std::io::{Write, stdout};
use std::net::UdpSocket;

const PORT: u16 = 1234;
const DELETE: u8 = 127;

fn main() {
    println!("Hello from the udp receiver");
    println!("Listening on {PORT}");

    assert_eq!(
        unsafe { libc::fcntl(0, libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );

    let socket = UdpSocket::bind(format!("0.0.0.0:{PORT}")).expect("bind must work");
    socket
        .set_nonblocking(true)
        .expect("nonblocking must work");

    let mut input = String::new();
    let mut last_sender = None;

    loop {
        let mut buffer = [0; 64];
        match socket.recv_from(&mut buffer) {
            Ok((count, src_addr)) => {
                last_sender = Some(src_addr);
                let text = std::str::from_utf8(&buffer[..count]).expect("Must be valid utf8");
                print!("{}", text);
                let _ = stdout().flush();
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("recv_from failed: {e}"),
        }

        let mut c = 0u8;
        let ret = unsafe { libc::read(0, &mut c as *mut u8 as *mut libc::c_void, 1) };
        if ret == 1 {
            match c {
                b'\r' | b'\n' => {
                    println!();
                    input.push(b'\n' as char);
                    if let Some(addr) = last_sender {
                        socket
                            .send_to(input.as_bytes(), addr)
                            .expect("send must work");
                    }
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
