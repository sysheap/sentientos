use serial_test::file_serial;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn boot() -> anyhow::Result<()> {
    QemuInstance::start().await?;
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
    let mut yaos = QemuInstance::start().await?;

    yaos.run_prog_waiting_for("exit", "shutting down system")
        .await?;

    assert!(yaos.wait_for_qemu_to_exit().await?.success());

    Ok(())
}

#[tokio::test]
async fn execute_program() -> anyhow::Result<()> {
    let mut yaos = QemuInstance::start().await?;

    let output = yaos.run_prog("prog1").await?;

    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn execute_same_program_twice() -> anyhow::Result<()> {
    let mut yaos = QemuInstance::start().await?;

    let expected = "Hello from Prog1\n";

    let output = yaos.run_prog("prog1").await?;
    assert_eq!(output, expected);

    let output = yaos.run_prog("prog1").await?;
    assert_eq!(output, expected);

    Ok(())
}

#[tokio::test]
async fn execute_different_programs() -> anyhow::Result<()> {
    let mut yaos = QemuInstance::start().await?;

    let output = yaos.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    let output = yaos.run_prog("prog2").await?;
    assert_eq!(output, "Hello from Prog2\n");

    Ok(())
}
