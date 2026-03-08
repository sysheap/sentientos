use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn dev_random_and_at_random() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    let output = solaya.run_prog("rng_test").await?;
    assert!(output.contains("OK dev_random"));
    assert!(output.contains("OK at_random"));
    Ok(())
}
