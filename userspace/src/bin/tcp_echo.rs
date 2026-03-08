use std::{
    io::{Read, Write},
    net::TcpListener,
};

const PORT: u16 = 1234;

fn main() {
    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).expect("bind must work");
    println!("TCP listening on {PORT}");

    let (mut stream, addr) = listener.accept().expect("accept must work");
    println!("Connection from {addr}");

    let mut buf = [0u8; 1024];
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        if stream.write_all(&buf[..n]).is_err() {
            break;
        }
    }
}
