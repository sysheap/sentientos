use crate::infra::qemu::QemuInstance;
use tokio::io::AsyncBufReadExt;

#[tokio::test]
async fn boot() -> anyhow::Result<()> {
    let mut instance = QemuInstance::start()?;

    let stdout = instance.stdout();

    let mut buffer = String::new();

    let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while stdout.read_line(&mut buffer).await? != 0 {
            if buffer.contains("### YaSH - Yet another Shell ###") {
                return Ok::<bool, anyhow::Error>(true);
            }
        }
        Ok(false)
    })
    .await?
    .unwrap();
    assert!(result);

    Ok(())
}
