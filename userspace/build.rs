fn main() {
    println!("cargo:rerun-if-changed=userspace/userspace.ld");
    println!("cargo:rustc-link-arg=-Tuserspace/userspace.ld");
}
