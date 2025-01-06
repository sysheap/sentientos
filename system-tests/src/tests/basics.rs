use serial_test::file_serial;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn boot_smp() -> anyhow::Result<()> {
    QemuInstance::start().await?;
    Ok(())
}

#[tokio::test]
async fn boot_single_core() -> anyhow::Result<()> {
    QemuInstance::start_with(QemuOptions::default().use_smp(false)).await?;
    Ok(())
}

#[file_serial]
#[tokio::test]
async fn boot_with_network() -> anyhow::Result<()> {
    QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;
    Ok(())
}

#[tokio::test]
async fn shutdown() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    sentientos
        .run_prog_waiting_for("exit", "shutting down system")
        .await?;

    assert!(sentientos.wait_for_qemu_to_exit().await?.success());

    Ok(())
}

#[tokio::test]
async fn execute_program() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    let output = sentientos.run_prog("prog1").await?;

    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn execute_same_program_twice() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    let expected = "Hello from Prog1\n";

    let output = sentientos.run_prog("prog1").await?;
    assert_eq!(output, expected);

    let output = sentientos.run_prog("prog1").await?;
    assert_eq!(output, expected);

    Ok(())
}

#[tokio::test]
async fn execute_different_programs() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    let output = sentientos.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    let output = sentientos.run_prog("prog2").await?;
    assert_eq!(output, "Hello from Prog2\n");

    Ok(())
}
