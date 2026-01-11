use std::time::{Duration, Instant};

use serial_test::file_serial;

use crate::infra::qemu::QemuInstance;

#[file_serial]
#[tokio::test]
async fn stress() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    // Spawn 8 concurrent processes to stress test the scheduler
    let start = Instant::now();
    sentientos
        .run_prog_waiting_for("stress 8", "Done!")
        .await?;
    let elapsed = start.elapsed();

    // Each loop instance runs 5 iterations with 1-second sleeps.
    // 8 processes running concurrently should complete in ~5-7 seconds.
    // If sequential, it would take 40+ seconds.
    assert!(
        elapsed >= Duration::from_secs(4),
        "Should take at least 4 seconds (5 iterations × 1 second sleep)"
    );
    assert!(
        elapsed < Duration::from_secs(15),
        "Should complete within 15 seconds if processes run concurrently"
    );

    Ok(())
}
