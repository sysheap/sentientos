extern crate alloc;

use std::{
    io::{Read, Write, stdout},
    string::String,
};

const DELETE: u8 = 127;

pub fn read_line() -> String {
    let mut input = String::new();
    loop {
        let mut buf = [0];
        let count = std::io::stdin().read(&mut buf).unwrap();
        if count == 0 {
            continue;
        }
        match buf[0] {
            b'\r' | b'\n' => {
                // Carriage return
                println!();
                break;
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
    input
}
