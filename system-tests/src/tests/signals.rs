use tokio::io::AsyncWriteExt;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn should_exit_program() -> anyhow::Result<()> {
    let mut solaya =
        QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    solaya
        .run_prog_waiting_for("udp", "Listening on 1234")
        .await?;

    solaya.ctrl_c_and_assert_prompt().await?;

    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn should_not_exit_sosh() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    solaya.stdin().write_all(&[0x03]).await?;
    solaya.stdin().flush().await?;

    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}
