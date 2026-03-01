use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn mutex_contention() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("mutex_test").await?;
    assert_eq!(output, "mutex_test passed: counter=4000\n");

    Ok(())
}
