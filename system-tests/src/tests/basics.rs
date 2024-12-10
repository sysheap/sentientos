use serial_test::file_serial;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn boot() -> anyhow::Result<()> {
    QemuInstance::start().await?;
    Ok(())
}

#[file_serial]
#[tokio::test]
async fn boot_with_network() -> anyhow::Result<()> {
    QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;
    Ok(())
}
