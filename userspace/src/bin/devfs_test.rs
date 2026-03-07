use std::io::{Read, Write};

fn main() {
    // Test 1: Write to /dev/null
    {
        let mut f = std::fs::File::create("/dev/null").expect("open /dev/null for write failed");
        f.write_all(b"discard this")
            .expect("write to /dev/null failed");
    }
    println!("OK null_write");

    // Test 2: Read from /dev/null (should return EOF immediately)
    {
        let mut f = std::fs::File::open("/dev/null").expect("open /dev/null for read failed");
        let mut buf = [0u8; 64];
        let n = f.read(&mut buf).expect("read from /dev/null failed");
        assert_eq!(n, 0, "/dev/null read should return 0 bytes");
    }
    println!("OK null_read");

    // Test 3: Read from /dev/zero (should return zero-filled bytes)
    {
        let mut f = std::fs::File::open("/dev/zero").expect("open /dev/zero for read failed");
        let mut buf = [0xFFu8; 64];
        let n = f.read(&mut buf).expect("read from /dev/zero failed");
        assert_eq!(n, 64, "/dev/zero should fill entire buffer");
        assert!(
            buf.iter().all(|&b| b == 0),
            "/dev/zero should return all zeros"
        );
    }
    println!("OK zero_read");

    // Test 4: Write to /dev/zero
    {
        let mut f = std::fs::File::create("/dev/zero").expect("open /dev/zero for write failed");
        f.write_all(b"discard this too")
            .expect("write to /dev/zero failed");
    }
    println!("OK zero_write");
}
