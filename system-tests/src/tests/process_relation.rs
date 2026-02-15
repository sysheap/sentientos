use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn getppid_returns_valid_parent() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    let output = sentientos.run_prog("getppid").await?;
    let ppid: u64 = output.trim().parse()?;
    assert!(ppid > 0, "Parent PID should be > 0, got {ppid}");

    Ok(())
}

#[tokio::test]
async fn wait_non_child_returns_error() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    let output = sentientos.run_prog("wait_non_child").await?;
    assert_eq!(output.trim(), "NotAChild");

    Ok(())
}

#[tokio::test]
async fn stress_with_parent_child_wait() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;
    sentientos.run_prog_waiting_for("stress 4", "Done!").await?;
    Ok(())
}
