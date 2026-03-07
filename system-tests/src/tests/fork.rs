use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn fork_basic() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("fork-test").await?;

    assert!(output.contains("child"), "expected child output: {output}");
    assert!(
        output.contains("parent waited"),
        "expected parent output: {output}"
    );

    Ok(())
}
