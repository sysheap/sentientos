use tokio::io::AsyncWriteExt;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn should_exit_program() -> anyhow::Result<()> {
    let mut sentientos =
        QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    sentientos
        .run_prog_waiting_for("udp", "Listening on 1234")
        .await?;

    sentientos.ctrl_c_and_assert_prompt().await?;

    let output = sentientos.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn should_not_exit_sesh() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    sentientos.stdin().write_all(&[0x03]).await?;
    sentientos.stdin().flush().await?;

    let output = sentientos.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}
