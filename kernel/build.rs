use std::{collections::BTreeMap, env, error::Error, io::Write, path::Path, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=qemu.ld");
    println!("cargo:rerun-if-changed=../userspace/");
    println!("cargo:rerun-if-changed=../common/");
    println!("cargo:rustc-link-arg-bin=kernel=-Tkernel/qemu.ld");

    if is_miri_execution() {
        return Ok(());
    }

    build_userspace_programs()?;
    generate_userspace_programs_include()?;
    Ok(())
}

fn is_miri_execution() -> bool {
    env::var_os("CARGO_CFG_MIRI").is_some()
}

fn generate_userspace_programs_include() -> Result<(), Box<dyn Error>> {
    const USERSPACE_PROGRAMS_PATH: &str = "../kernel/src/autogenerated/userspace_programs.rs";

    let mut userspace_programs = std::fs::File::create(USERSPACE_PROGRAMS_PATH)?;

    writeln!(userspace_programs, "use common::include_bytes_align_as;\n")?;

    // Use BTreeMap to have the program names in a sorted order
    let mut programs: BTreeMap<String, String> = BTreeMap::new();

    for entry in std::fs::read_dir("../kernel/compiled_userspace")? {
        let entry = entry?;
        let path = entry.path();
        let original_file_name = path.file_name().unwrap().to_str().unwrap();
        let file_name = original_file_name.to_uppercase();

        programs.insert(original_file_name.to_owned(), file_name.clone());

        writeln!(
            userspace_programs,
            "pub static {file_name}: &[u8] = include_bytes_align_as!(u64, \"../../compiled_userspace/{original_file_name}\");"
        )?;
    }

    writeln!(userspace_programs)?;
    write!(
        userspace_programs,
        "pub static PROGRAMS: &[(&str, &[u8])] = &["
    )?;
    for (original_file_name, file_name) in programs {
        write!(
            userspace_programs,
            "(\"{original_file_name}\", {file_name}),"
        )?;
    }
    write!(userspace_programs, "];")?;

    drop(userspace_programs);

    // Format the newly generated file
    Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg(USERSPACE_PROGRAMS_PATH)
        .spawn()?
        .wait()?;

    Ok(())
}

fn build_userspace_programs() -> Result<(), Box<dyn Error>> {
    let compiled_userspace_path = Path::new("../kernel/compiled_userspace");

    let _ = std::fs::remove_dir_all(compiled_userspace_path);

    let mut command = Command::new("cargo");
    command.current_dir("../userspace");

    command.args([
        "build",
        "--bins",
        "--target-dir",
        "../../target-userspace",
        "--artifact-dir",
        compiled_userspace_path.to_str().unwrap(),
        "-Z",
        "unstable-options",
        "--release",
    ]);

    let status = command.status()?;
    if !status.success() {
        return Err(From::from("Failed to build userspace programs"));
    }

    Ok(())
}
