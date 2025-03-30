use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn connect4() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    sentientos
        .run_prog_waiting_for("connect4\n10\nc", "Calculating moves...")
        .await?;

    sentientos.ctrl_c_and_assert_prompt().await?;

    Ok(())
}
