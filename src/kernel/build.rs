use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=qemu.ld");
    println!("cargo:rerun-if-changed=../userspace/");
    println!("cargo:rustc-link-arg-bin=kernel=-Tsrc/kernel/qemu.ld");

    build_userspace_programs()?;

    Ok(())
}

fn build_userspace_programs() -> Result<(), Box<dyn Error>> {
    let profile = std::env::var("PROFILE")?;

    let mut command = Command::new("cargo");
    command.current_dir("../userspace");

    command.args([
        "build",
        "--bins",
        "--target-dir",
        "../../target-userspace",
        "--out-dir",
        "../kernel/compiled_userspace",
        "-Z",
        "unstable-options",
    ]);

    if profile == "release" {
        command.arg("--release");
    }

    let status = command.status()?;
    if !status.success() {
        return Err(From::from("Failed to build userspace programs"));
    }

    Ok(())
}
