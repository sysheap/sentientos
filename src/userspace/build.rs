fn main() {
    println!("cargo:rerun-if-changed=src/userspace/userspace.ld");
}
