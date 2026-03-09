use std::time::Instant;

fn main() {
    let start = Instant::now();
    let elapsed = start.elapsed();
    println!("clock OK: {:?}", elapsed);

    // Busy-wait a bit to verify clock progresses
    let start2 = Instant::now();
    for _ in 0..100_000 {
        core::hint::black_box(0);
    }
    let elapsed2 = start2.elapsed();
    assert!(elapsed2 > std::time::Duration::ZERO);
    println!("clock progression OK");
}
