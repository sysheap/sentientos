fn main() {
    println!("cargo:rerun-if-changed=src/userspace/userspace.ld");
    println!("cargo:rustc-link-arg=-Tsrc/userspace/userspace.ld");
}
