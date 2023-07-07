fn main() {
    println!("cargo:rerun-if-changed=kernel/qemu.ld");

    println!("cargo:rustc-link-arg-bin=kernel=-Tkernel/qemu.ld");
}
