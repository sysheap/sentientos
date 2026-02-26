use crate::infra::qemu::QemuInstance;
use qemu_infra::PROMPT;

#[tokio::test]
async fn background_execution() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    solaya.write_and_wait_for("sleep 10 &\n", PROMPT).await?;
    solaya
        .write_and_wait_for("prog1\n", "Hello from Prog1")
        .await?;
    Ok(())
}

#[tokio::test]
async fn help_lists_programs() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    solaya.write_and_wait_for("help\n", "sosh").await?;
    Ok(())
}

#[tokio::test]
async fn execute_nonexistent_program() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    let output = solaya.run_prog("nonexistent").await?;
    assert!(output.contains("Error executing program"));
    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");
    Ok(())
}
