use std::{
    io::{Read, Write, stdout},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

const PORT: u16 = 1234;
const DELETE: u8 = 127;

/// Stores IPv4 addr + port in a single AtomicU64 (high 32 = ip, low 16 = port).
/// Zero means no address stored yet.
struct AtomicSocketAddr(AtomicU64);

impl AtomicSocketAddr {
    const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    fn store(&self, addr: SocketAddr) {
        let SocketAddr::V4(v4) = addr else {
            panic!("IPv6 not supported");
        };
        let packed = ((u32::from(*v4.ip()) as u64) << 16) | v4.port() as u64;
        self.0.store(packed, Ordering::Relaxed);
    }

    fn load(&self) -> Option<SocketAddr> {
        let packed = self.0.load(Ordering::Relaxed);
        if packed == 0 {
            return None;
        }
        let ip = (packed >> 16) as u32;
        let port = packed as u16;
        Some(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(ip), port)))
    }
}

fn main() {
    println!("Hello from the udp receiver");
    println!("Listening on {PORT}");

    let socket = Arc::new(UdpSocket::bind(format!("0.0.0.0:{PORT}")).expect("bind must work"));
    // Kernel recvfrom always returns EAGAIN when no data (blocking not implemented),
    // so we must use non-blocking mode and poll.
    socket.set_nonblocking(true).expect("nonblocking must work");

    let last_sender = Arc::new(AtomicSocketAddr::new());

    let recv_socket = Arc::clone(&socket);
    let recv_sender = Arc::clone(&last_sender);
    thread::spawn(move || {
        let mut buffer = [0; 64];
        loop {
            match recv_socket.recv_from(&mut buffer) {
                Ok((count, src_addr)) => {
                    recv_sender.store(src_addr);
                    let text = std::str::from_utf8(&buffer[..count]).expect("Must be valid utf8");
                    print!("{}", text);
                    let _ = stdout().flush();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("recv_from failed: {e}"),
            }
        }
    });

    let mut input = String::new();
    loop {
        let mut buf = [0];
        let count = std::io::stdin().read(&mut buf).unwrap();
        if count == 0 {
            break;
        }
        match buf[0] {
            b'\r' | b'\n' => {
                println!();
                input.push('\n');
                if let Some(addr) = last_sender.load() {
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
                assert!(buf[0].is_ascii());
                let result = buf[0] as char;
                input.push(result);
                print!("{}", result);
                let _ = stdout().flush();
            }
        }
    }
}
