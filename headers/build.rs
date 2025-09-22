use std::{
    env,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use bindgen::callbacks::ParseCallbacks;

const SYSCALL_PREFIX: &str = "__NR_";

#[derive(Debug, Default, Clone)]
struct SyscallReaderCallback {
    syscalls: Arc<Mutex<Vec<String>>>,
}

impl SyscallReaderCallback {
    fn get_syscalls(&self) -> Vec<String> {
        let lg = self.syscalls.lock().unwrap();
        lg.clone()
    }
}

impl ParseCallbacks for SyscallReaderCallback {
    fn int_macro(&self, name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
        if name.starts_with(SYSCALL_PREFIX) {
            let mut lg = self.syscalls.lock().unwrap();
            lg.push(name.replace(SYSCALL_PREFIX, ""));
            return Some(bindgen::callbacks::IntKind::Custom {
                name: "usize",
                is_signed: false,
            });
        }
        None
    }

    fn item_name(&self, item_info: bindgen::callbacks::ItemInfo) -> Option<String> {
        if item_info.name.starts_with(SYSCALL_PREFIX) {
            return Some(format!(
                "SYSCALL_NR_{}",
                item_info.name.replace(SYSCALL_PREFIX, "").to_uppercase()
            ));
        }
        None
    }
}

#[derive(Debug)]
struct ErrnoCallback;

impl ParseCallbacks for ErrnoCallback {
    fn int_macro(&self, _name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
        Some(bindgen::callbacks::IntKind::Custom {
            name: "isize",
            is_signed: true,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    generate_syscall_nr_file(&out_path)?;
    generate_syscall_types(&out_path)?;
    generate_error_types(&out_path)?;
    Ok(())
}

fn default_bindgen_builder() -> bindgen::Builder {
    bindgen::Builder::default()
        .clang_arg("-Ilinux_headers/include")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .use_core()
}

fn generate_syscall_types(out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bindings = default_bindgen_builder()
        .header("linux_headers/include/asm-generic/poll.h")
        .header("linux_headers/include/asm-generic/signal.h")
        .header("linux_headers/include/linux/time.h")
        .generate()?;
    let syscall_file_path = out_path.join("syscall_types.rs");
    bindings.write_to_file(syscall_file_path.clone())?;
    Ok(())
}

fn generate_error_types(out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bindings = default_bindgen_builder()
        .header("linux_headers/include/asm-generic/errno.h")
        .parse_callbacks(Box::new(ErrnoCallback))
        .generate()?;
    let syscall_file_path = out_path.join("errno.rs");
    bindings.write_to_file(syscall_file_path.clone())?;
    Ok(())
}

fn generate_syscall_nr_file(out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let syscall_type_changer = SyscallReaderCallback::default();
    let bindings = default_bindgen_builder()
        .header("linux_headers/include/asm/unistd.h")
        .parse_callbacks(Box::new(syscall_type_changer.clone()))
        .generate()?;

    let syscall_file_path = out_path.join("syscalls.rs");
    bindings.write_to_file(syscall_file_path.clone())?;

    let mut syscall_names_file = File::options()
        .append(true)
        .open(syscall_file_path.clone())?;

    let syscalls = syscall_type_changer.get_syscalls();
    writeln!(
        syscall_names_file,
        "pub const SYSCALL_NAMES: [(usize, &str); {}] = [",
        syscalls.len()
    )?;
    for name in syscalls {
        writeln!(
            syscall_names_file,
            "(SYSCALL_NR_{}, \"{name}\"),",
            name.to_uppercase()
        )?;
    }
    writeln!(syscall_names_file, "];")?;

    drop(syscall_names_file);

    // Format newly generated file
    Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg(syscall_file_path)
        .spawn()?
        .wait()?;
    Ok(())
}
