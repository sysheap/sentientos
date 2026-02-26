use std::time::{Duration, Instant};

use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn sleep() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    {
        let start = Instant::now();
        solaya.run_prog("sleep 0").await?;
        let duration = start.elapsed();
        assert!(duration >= Duration::from_secs(0));
    }
    {
        let start = Instant::now();
        solaya.run_prog("sleep 1").await?;
        let duration = start.elapsed();
        assert!(duration >= Duration::from_secs(1));
    }

    Ok(())
}
