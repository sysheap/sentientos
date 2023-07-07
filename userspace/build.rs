fn main() {
    println!("cargo:rerun-if-changed=userspace/userspace.ld");
}
