fn main() {
    println!("cargo:rerun-if-changed=src/kernel/qemu.ld");

    println!("cargo:rustc-link-arg-bin=kernel=-Tsrc/kernel/qemu.ld");
}
