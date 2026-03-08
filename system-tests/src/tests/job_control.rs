use qemu_infra::PROMPT;
use tokio::io::AsyncWriteExt;

use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn ctrl_d_exits_cat() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    // Start cat
    solaya.stdin().write_all(b"cat\n").await?;
    solaya.stdin().flush().await?;
    solaya.stdout().assert_read_until("cat\n").await?;

    // Send Ctrl+D on empty line to signal EOF
    solaya.stdin().write_all(&[0x04]).await?;
    solaya.stdin().flush().await?;

    // cat should exit and shell prompt should return
    solaya.stdout().assert_read_until(PROMPT).await?;

    // Verify shell still works
    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn ctrl_z_stops_process() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    // Start cat
    solaya.stdin().write_all(b"cat\n").await?;
    solaya.stdin().flush().await?;
    solaya.stdout().assert_read_until("cat\n").await?;

    // Wait for cat to start and set up the foreground process group.
    // The child calls setpgid + ioctl(TIOCSPGRP) before execve.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Send Ctrl+Z to suspend cat
    solaya.stdin().write_all(&[0x1a]).await?;
    solaya.stdin().flush().await?;

    // Shell should print ^Z, then the stopped notification, then prompt
    solaya.stdout().assert_read_until(PROMPT).await?;

    // Verify shell still works with a stopped background process
    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn fg_resumes_stopped_process() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    // Start cat
    solaya.stdin().write_all(b"cat\n").await?;
    solaya.stdin().flush().await?;
    solaya.stdout().assert_read_until("cat\n").await?;

    // Wait for cat to start and set up the foreground process group
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Suspend cat with Ctrl+Z
    solaya.stdin().write_all(&[0x1a]).await?;
    solaya.stdin().flush().await?;
    solaya.stdout().assert_read_until(PROMPT).await?;

    // Resume cat with fg
    solaya.stdin().write_all(b"fg\n").await?;
    solaya.stdin().flush().await?;
    solaya.stdout().assert_read_until("fg\n").await?;

    // cat is now in foreground again — send Ctrl+D to exit
    solaya.stdin().write_all(&[0x04]).await?;
    solaya.stdin().flush().await?;

    // cat exits, shell prompt returns
    solaya.stdout().assert_read_until(PROMPT).await?;

    // Verify shell still works
    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}
