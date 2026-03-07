use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn cat_proc_version() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    let output = solaya.run_prog("cat /proc/version").await?;
    assert_eq!(output.trim(), "Solaya 0.1.0");
    Ok(())
}

#[tokio::test]
async fn touch_and_cat() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    solaya.run_prog("touch /tmp/test").await?;
    let output = solaya.run_prog("cat /tmp/test").await?;
    assert_eq!(output, "");
    Ok(())
}

#[tokio::test]
async fn rm_file() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    solaya.run_prog("touch /tmp/x").await?;
    solaya.run_prog("rm /tmp/x").await?;
    Ok(())
}

#[tokio::test]
async fn vfs_roundtrip() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;
    let output = solaya.run_prog("vfs-test").await?;
    assert!(output.contains("OK create_and_write"));
    assert!(output.contains("OK read_back"));
    assert!(output.contains("OK proc_version"));
    assert!(output.contains("OK remove"));
    assert!(output.contains("OK gone"));
    Ok(())
}
