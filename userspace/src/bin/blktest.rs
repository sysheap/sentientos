use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

fn main() {
    let mut file = File::open("/dev/vda").expect("Failed to open /dev/vda");

    let mut buf = [0u8; 512];
    let n = file.read(&mut buf).expect("Failed to read");
    assert_eq!(n, 512, "Expected 512 bytes");

    // Print first 16 bytes as hex
    for b in &buf[..16] {
        print!("{:02x}", b);
    }
    println!();
    println!("OK blk_read");

    // Test reading at an offset
    file.seek(SeekFrom::Start(512)).expect("Failed to seek");
    let mut buf2 = [0u8; 512];
    let n2 = file.read(&mut buf2).expect("Failed to read sector 1");
    assert_eq!(n2, 512, "Expected 512 bytes from sector 1");

    for b in &buf2[..16] {
        print!("{:02x}", b);
    }
    println!();
    println!("OK blk_seek");
}
