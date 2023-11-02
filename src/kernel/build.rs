use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/kernel/qemu.ld");
    println!("cargo:rerun-if-changed=src/userspace/");
    println!("cargo:rustc-link-arg-bin=kernel=-Tsrc/kernel/qemu.ld");

    build_userspace_programs()?;

    Ok(())
}

fn build_userspace_programs() -> Result<(), Box<dyn Error>> {
    let status = Command::new("cargo")
        .args([
            "install",
            "--path",
            ".",
            "--root",
            "../kernel/compiled_userspace",
        ])
        .current_dir("../userspace")
        .status()?;

    if !status.success() {
        return Err(From::from("Failed to build userspace programs"));
    }

    Ok(())
}
