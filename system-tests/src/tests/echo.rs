use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn execute_program() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;

    let output = sentientos.run_prog("echo").await?;
    assert_eq!(output, "\n");

    let output = sentientos.run_prog("echo 1 2 3").await?;
    assert_eq!(output, "1 2 3\n");

    let output = sentientos.run_prog("echo 1     2     3 text").await?;
    assert_eq!(output, "1 2 3 text\n");
    Ok(())
}
