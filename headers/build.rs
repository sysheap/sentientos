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

#[derive(Debug, Clone, Default)]
struct ErrnoCallback {
    errnos: Arc<Mutex<Vec<(String, isize)>>>,
}

impl ParseCallbacks for ErrnoCallback {
    fn int_macro(&self, name: &str, value: i64) -> Option<bindgen::callbacks::IntKind> {
        // Ignore duplicate definitions
        if ["EWOULDBLOCK", "EDEADLOCK"].contains(&name) {
            return None;
        }
        self.errnos
            .lock()
            .unwrap()
            .push((name.into(), value as isize));
        None
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
    let errno_callback = ErrnoCallback::default();
    let _ = default_bindgen_builder()
        .header("linux_headers/include/asm-generic/errno.h")
        .parse_callbacks(Box::new(errno_callback.clone()))
        .generate()?;
    let errno_path = out_path.join("errno.rs");
    let mut errno_file = File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .open(errno_path.clone())?;

    writeln!(errno_file, "#[repr(isize)]")?;
    writeln!(errno_file, "#[derive(Debug, PartialEq, Eq, Copy, Clone)]")?;
    writeln!(errno_file, "pub enum Errno {{")?;

    for (error, value) in errno_callback.errnos.lock().unwrap().iter() {
        writeln!(errno_file, "{error} = {value},")?;
    }

    writeln!(errno_file, "}}")?;

    drop(errno_file);
    format_file(errno_path)?;

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

    let lg = syscall_type_changer.syscalls.lock().unwrap();

    writeln!(
        syscall_names_file,
        "pub const SYSCALL_NAMES: [(usize, &str); {}] = [",
        lg.len()
    )?;
    for name in lg.iter() {
        writeln!(
            syscall_names_file,
            "(SYSCALL_NR_{}, \"{name}\"),",
            name.to_uppercase()
        )?;
    }
    writeln!(syscall_names_file, "];")?;

    drop(syscall_names_file);

    format_file(syscall_file_path)?;
    Ok(())
}

fn format_file(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg(path)
        .spawn()?
        .wait()?;
    Ok(())
}
