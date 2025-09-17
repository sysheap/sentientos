use std::{env, path::PathBuf};

use bindgen::callbacks::ParseCallbacks;

const SYSCALL_PREFIX: &str = "__NR_";

#[derive(Debug)]
struct SyscallTypeChanger;

impl ParseCallbacks for SyscallTypeChanger {
    fn int_macro(&self, _name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
        if _name.starts_with(SYSCALL_PREFIX) {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bindings = bindgen::Builder::default()
        .header("linux_headers/include/asm/unistd.h")
        .clang_arg("-Ilinux_headers/include")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(SyscallTypeChanger))
        .generate()?;

    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("syscalls.rs"))?;
    Ok(())
}
