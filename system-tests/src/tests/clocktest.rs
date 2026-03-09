use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn clock_gettime_returns_nonzero() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    let output = solaya.run_prog("clocktest").await?;
    assert!(
        output.contains("clock OK"),
        "clock_gettime failed: {}",
        output
    );
    assert!(
        output.contains("clock progression OK"),
        "clock not progressing: {}",
        output
    );
    Ok(())
}
