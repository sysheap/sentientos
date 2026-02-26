use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn echo() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("echo").await?;
    assert_eq!(output, "\n");

    let output = solaya.run_prog("echo 1 2 3").await?;
    assert_eq!(output, "1 2 3\n");

    let output = solaya.run_prog("echo 1     2     3 text").await?;
    assert_eq!(output, "1 2 3 text\n");
    Ok(())
}

#[tokio::test]
async fn r#true() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("true").await?;
    assert_eq!(output, "");

    Ok(())
}

#[tokio::test]
async fn r#false() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("false").await?;
    assert_eq!(output, "");

    Ok(())
}
