use serial_test::file_serial;
use tokio::io::AsyncWriteExt;

use crate::infra::{
    qemu::{QemuInstance, QemuOptions},
    PROMPT,
};

#[file_serial]
#[tokio::test]
async fn should_exit_program() -> anyhow::Result<()> {
    let mut yaros = QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    yaros
        .run_prog_waiting_for("udp", "Listening on 1234")
        .await?;

    yaros.stdin().write_all(&[0x03]).await?;

    yaros.stdout().assert_read_until(PROMPT).await;
    let output = yaros.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn should_not_exit_yash() -> anyhow::Result<()> {
    let mut yaros = QemuInstance::start().await?;

    yaros.stdin().write_all(&[0x03]).await?;

    let output = yaros.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}
