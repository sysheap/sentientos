use std::io::Write;

use crate::infra::qemu::{QemuInstance, QemuOptions};

fn create_test_disk(path: &std::path::Path) {
    let mut file = std::fs::File::create(path).expect("create disk image");
    let mut sector0 = [0u8; 512];
    sector0[..8].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
    file.write_all(&sector0).expect("write sector 0");

    let mut sector1 = [0u8; 512];
    sector1[..4].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    file.write_all(&sector1).expect("write sector 1");

    file.write_all(&vec![0u8; 1024 * 1024 - 1024])
        .expect("pad disk");
    file.flush().expect("flush disk");
}

#[tokio::test]
async fn block_read() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let disk_path = dir.path().join("disk.img");
    create_test_disk(&disk_path);

    let mut solaya =
        QemuInstance::start_with(QemuOptions::default().block_device(disk_path)).await?;

    let output = solaya.run_prog("blktest").await?;
    assert!(
        output.contains("deadbeefcafebabe"),
        "Expected first sector pattern, got: {}",
        output
    );
    assert!(
        output.contains("OK blk_read"),
        "Expected OK blk_read, got: {}",
        output
    );
    assert!(
        output.contains("11223344"),
        "Expected second sector pattern, got: {}",
        output
    );
    assert!(
        output.contains("OK blk_seek"),
        "Expected OK blk_seek, got: {}",
        output
    );

    Ok(())
}
