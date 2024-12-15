use serial_test::file_serial;
use tokio::io::AsyncWriteExt;

use crate::infra::{
    qemu::{QemuInstance, QemuOptions},
    PROMPT,
};

#[file_serial]
#[tokio::test]
async fn should_exit_program() -> anyhow::Result<()> {
    let mut yaos = QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    yaos.run_prog_waiting_for("udp", "Listening on 1234")
        .await?;

    yaos.stdin().write_all(&[0x03]).await?;

    yaos.stdout().assert_read_until(PROMPT).await;
    let output = yaos.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn should_not_exit_yash() -> anyhow::Result<()> {
    let mut yaos = QemuInstance::start().await?;

    yaos.stdin().write_all(&[0x03]).await?;

    let output = yaos.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}
