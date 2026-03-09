use std::{fs::OpenOptions, io::Write};

fn main() {
    let mut fb = OpenOptions::new()
        .write(true)
        .open("/dev/fb0")
        .expect("open fb0");
    // Write a red pixel at position (0,0): XRGB8888 format
    let pixel: [u8; 4] = [0, 0, 0xFF, 0];
    fb.write_all(&pixel).expect("write pixel");
    println!("fb write OK");
}
