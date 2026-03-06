use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn getppid_returns_valid_parent() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("getppid").await?;
    let ppid: u64 = output.trim().parse()?;
    assert!(ppid > 0, "Parent PID should be > 0, got {ppid}");

    Ok(())
}

#[tokio::test]
async fn pgid_and_sid_syscalls() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("pgid-test").await?;
    assert!(
        output.contains("OK"),
        "pgid-test should print OK, got: {output}"
    );
    assert!(output.contains("pgid="), "should print pgid");
    assert!(output.contains("sid="), "should print sid");

    Ok(())
}

#[tokio::test]
async fn setpgid_creates_new_process_group() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("pgid-child setpgid").await?;
    assert!(
        output.contains("OK"),
        "pgid-child setpgid should print OK, got: {output}"
    );
    assert!(output.contains("child_pgid="), "should print child_pgid");

    Ok(())
}

#[tokio::test]
async fn setsid_creates_new_session() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("pgid-child setsid").await?;
    assert!(
        output.contains("OK"),
        "pgid-child setsid should print OK, got: {output}"
    );
    assert!(output.contains("child_sid="), "should print child_sid");

    Ok(())
}
