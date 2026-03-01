use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn getppid_returns_valid_parent() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("getppid").await?;
    let ppid: u64 = output.trim().parse()?;
    assert!(ppid > 0, "Parent PID should be > 0, got {ppid}");

    Ok(())
}
