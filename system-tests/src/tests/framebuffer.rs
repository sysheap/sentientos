use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn framebuffer_device_exists() -> anyhow::Result<()> {
    let mut solaya =
        QemuInstance::start_with(QemuOptions::default().framebuffer(true)).await?;
    let output = solaya.run_prog("fbtest").await?;
    assert!(
        output.contains("fb write OK"),
        "framebuffer test failed: {}",
        output
    );
    Ok(())
}
