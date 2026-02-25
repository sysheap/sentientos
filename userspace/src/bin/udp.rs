use std::{
    io::{Write, stdout},
    net::UdpSocket,
};

const PORT: u16 = 1234;
const DELETE: u8 = 127;
const F_SETFL: usize = 4;
const O_NONBLOCK: usize = 0o4000;

fn raw_syscall3(nr: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!("ecall",
            in("a7") nr,
            in("a0") a0,
            in("a1") a1,
            in("a2") a2,
            lateout("a0") ret,
        );
    }
    ret
}

fn set_stdin_nonblocking() {
    assert_eq!(raw_syscall3(25, 0, F_SETFL, O_NONBLOCK), 0); // __NR_fcntl = 25
}

fn try_read_byte() -> Option<u8> {
    let mut c = 0u8;
    let ret = raw_syscall3(63, 0, &mut c as *mut u8 as usize, 1); // __NR_read = 63
    if ret == 1 { Some(c) } else { None }
}

fn main() {
    println!("Hello from the udp receiver");
    println!("Listening on {PORT}");

    set_stdin_nonblocking();

    let socket = UdpSocket::bind(format!("0.0.0.0:{PORT}")).expect("bind must work");
    socket.set_nonblocking(true).expect("nonblocking must work");

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

        if let Some(c) = try_read_byte() {
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
