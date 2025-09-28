use std::time::{Duration, Instant};

use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn sleep() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    {
        let start = Instant::now();
        sentientos.run_prog("sleep 0").await?;
        let duration = start.elapsed();
        assert!(duration >= Duration::from_secs(0));
    }
    {
        let start = Instant::now();
        sentientos.run_prog("sleep 1").await?;
        let duration = start.elapsed();
        assert!(duration >= Duration::from_secs(1));
    }

    Ok(())
}
