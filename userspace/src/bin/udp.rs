use std::{
    io::{Read, Write, stdout},
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    thread,
};

const PORT: u16 = 1234;
const DELETE: u8 = 127;

fn main() {
    println!("Hello from the udp receiver");
    println!("Listening on {PORT}");

    let socket = Arc::new(UdpSocket::bind(format!("0.0.0.0:{PORT}")).expect("bind must work"));
    // Kernel recvfrom always returns EAGAIN when no data (blocking not implemented),
    // so we must use non-blocking mode and poll.
    socket.set_nonblocking(true).expect("nonblocking must work");

    let last_sender: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));

    let recv_socket = Arc::clone(&socket);
    let recv_sender = Arc::clone(&last_sender);
    thread::spawn(move || {
        let mut buffer = [0; 64];
        loop {
            match recv_socket.recv_from(&mut buffer) {
                Ok((count, src_addr)) => {
                    *recv_sender.lock().unwrap() = Some(src_addr);
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
                if let Some(addr) = *last_sender.lock().unwrap() {
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
