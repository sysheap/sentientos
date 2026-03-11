use std::{fs::OpenOptions, io::Read, os::unix::fs::OpenOptionsExt};

const O_NONBLOCK: i32 = 0x800;

fn main() {
    println!("Opening /dev/keyboard0...");
    let mut kb = match OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open("/dev/keyboard0")
    {
        Ok(f) => {
            println!("OK - keyboard device opened");
            f
        }
        Err(e) => {
            println!("FAIL: {}", e);
            return;
        }
    };

    println!("Waiting for keyboard events (press keys in QEMU window)...");
    let mut buf = [0u8; 128];
    let mut total = 0u32;
    let mut poll_count = 0u64;
    loop {
        match kb.read(&mut buf) {
            Ok(n) => {
                let event_count = n / 8;
                for i in 0..event_count {
                    let off = i * 8;
                    let event_type = u16::from_le_bytes([buf[off], buf[off + 1]]);
                    let code = u16::from_le_bytes([buf[off + 2], buf[off + 3]]);
                    let value = u32::from_le_bytes([
                        buf[off + 4],
                        buf[off + 5],
                        buf[off + 6],
                        buf[off + 7],
                    ]);
                    total += 1;
                    println!(
                        "Event #{}: type={} code={} value={} (polls={})",
                        total, event_type, code, value, poll_count
                    );
                }
                poll_count = 0;
            }
            Err(_) => {
                poll_count += 1;
                if poll_count.is_multiple_of(5_000_000) {
                    println!(
                        "Still polling... ({} polls, {} events so far)",
                        poll_count, total
                    );
                }
            }
        }
    }
}
