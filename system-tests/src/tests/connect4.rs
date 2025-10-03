use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn connect4() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    sentientos
        .run_prog_waiting_for("connect4", "search depth:")
        .await?;

    sentientos.write_and_wait_for("10\n", "(h)uman").await?;

    sentientos
        .write_and_wait_for("c\n", "Calculating moves...")
        .await?;

    sentientos.ctrl_c_and_assert_prompt().await?;

    Ok(())
}
