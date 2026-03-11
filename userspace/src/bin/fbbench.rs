use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    time::Instant,
};

fn main() {
    let mut fb = OpenOptions::new()
        .write(true)
        .open("/dev/fb0")
        .expect("open fb0");

    let frame_size = 640 * 400 * 4; // 640x400 XRGB8888
    let y_offset = (480 - 400) / 2;
    let offset = y_offset * 640 * 4;
    let buf = vec![0x42u8; frame_size];

    println!(
        "Benchmarking framebuffer write ({} bytes per frame)...",
        frame_size
    );

    let total_start = Instant::now();
    let mut frames = 0u32;

    for i in 0..20 {
        let start = Instant::now();
        fb.seek(SeekFrom::Start(offset as u64)).expect("seek");
        fb.write_all(&buf).expect("write");
        let elapsed = start.elapsed();
        frames += 1;
        println!(
            "Frame {}: {}.{:03}ms",
            i,
            elapsed.as_millis(),
            elapsed.as_micros() % 1000
        );
    }

    let total = total_start.elapsed();
    let avg_ms = total.as_millis() / u128::from(frames);
    let fps = 1000u128.checked_div(avg_ms).unwrap_or(999);
    println!(
        "Average: {}ms/frame, ~{} FPS ({} frames in {}.{:03}s)",
        avg_ms,
        fps,
        frames,
        total.as_secs(),
        total.as_millis() % 1000
    );
}
