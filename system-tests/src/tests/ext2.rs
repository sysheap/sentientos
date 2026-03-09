use std::path::Path;
use std::process::Command;

use crate::infra::qemu::{QemuInstance, QemuOptions};

fn find_mkfs_ext2() -> Option<String> {
    // Try PATH first
    if Command::new("mkfs.ext2")
        .arg("-V")
        .output()
        .is_ok_and(|o| o.status.success() || !o.stderr.is_empty())
    {
        return Some("mkfs.ext2".into());
    }
    // Try nix store paths
    for entry in std::fs::read_dir("/nix/store").ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.contains("e2fsprogs") && name_str.ends_with("-bin") {
            let candidate = entry.path().join("bin/mkfs.ext2");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn create_ext2_image(path: &Path, files: &[(&str, &str)], dirs: &[&str]) {
    let mkfs = find_mkfs_ext2().expect("mkfs.ext2 not found; install e2fsprogs");

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

    let output = Command::new(&mkfs)
        .args(["-d", &root.to_string_lossy()])
        .arg("-F")
        .arg(path)
        .arg("4096")
        .output()
        .expect("run mkfs.ext2");
    assert!(
        output.status.success(),
        "mkfs.ext2 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn ext2_read_file() -> anyhow::Result<()> {
    if find_mkfs_ext2().is_none() {
        eprintln!("SKIP: mkfs.ext2 not found");
        return Ok(());
    }

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
    if find_mkfs_ext2().is_none() {
        eprintln!("SKIP: mkfs.ext2 not found");
        return Ok(());
    }

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
