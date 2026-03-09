use std::{path::Path, process::Command};

use crate::infra::qemu::{QemuInstance, QemuOptions};

fn create_ext2_image(path: &Path, files: &[(&str, &str)], dirs: &[&str]) {
    let dir = tempfile::tempdir().expect("create tmpdir for ext2 root");
    let root = dir.path();

    for d in dirs {
        std::fs::create_dir_all(root.join(d)).expect("create subdir");
    }
    for (name, content) in files {
        let file_path = root.join(name);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(&file_path, content).expect("write file");
    }

    let output = Command::new("mkfs.ext2")
        .args(["-d", &root.to_string_lossy()])
        .arg("-F")
        .arg(path)
        .arg("4096")
        .output()
        .expect("run mkfs.ext2 (install e2fsprogs or enter nix shell)");
    assert!(
        output.status.success(),
        "mkfs.ext2 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn ext2_read_file() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let img = dir.path().join("ext2.img");
    create_ext2_image(
        &img,
        &[
            ("hello.txt", "Hello from ext2!"),
            ("subdir/nested.txt", "Nested content"),
        ],
        &["subdir"],
    );

    let mut solaya = QemuInstance::start_with(QemuOptions::default().block_device(img)).await?;
    let output = solaya.run_prog("ext2test").await?;

    assert!(
        output.contains("FILE:Hello from ext2!"),
        "Expected file content, got: {}",
        output
    );
    assert!(
        output.contains("OK read_file"),
        "Expected OK read_file, got: {}",
        output
    );
    assert!(
        output.contains("OK readdir"),
        "Expected OK readdir, got: {}",
        output
    );
    assert!(
        output.contains("NESTED:Nested content"),
        "Expected nested content, got: {}",
        output
    );
    assert!(
        output.contains("OK nested_read"),
        "Expected OK nested_read, got: {}",
        output
    );
    assert!(
        output.contains("OK write_erofs"),
        "Expected write EROFS, got: {}",
        output
    );
    assert!(
        output.contains("OK create_erofs"),
        "Expected create EROFS, got: {}",
        output
    );

    Ok(())
}

#[tokio::test]
async fn ext2_ls_mnt() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let img = dir.path().join("ext2.img");
    create_ext2_image(&img, &[("testfile.txt", "abc")], &[]);

    let mut solaya = QemuInstance::start_with(QemuOptions::default().block_device(img)).await?;
    let output = solaya.run_prog("ls-test /mnt").await?;

    assert!(
        output.contains("testfile.txt"),
        "Expected testfile.txt in ls output, got: {}",
        output
    );

    Ok(())
}
