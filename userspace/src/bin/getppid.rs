extern crate userspace;

fn main() {
    let ppid = std::os::unix::process::parent_id();
    println!("{ppid}");
}
