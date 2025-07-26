use std::{env, path::PathBuf};

type BoxedResult<T = ()> = Result<T, Box<dyn core::error::Error>>;

fn main() -> BoxedResult {
    build_newlib_bindings()?;
    Ok(())
}

fn build_newlib_bindings() -> BoxedResult {
    let bindings = bindgen::Builder::default()
        .header("newlib_includes.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_arg("--sysroot=../toolchain/sysroot/")
        .clang_arg("-I../toolchain/sysroot/usr/include/")
        .clang_arg("--target=riscv64")
        .clang_arg("-D_LIBC")
        .use_core()
        .generate()?;

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}
