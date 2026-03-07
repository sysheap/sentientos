use std::io::Write;

fn main() {
    // Test 1: Write to a file in /tmp
    let path = "/tmp/vfs-test-file";
    let content = b"hello vfs";
    {
        let mut f = std::fs::File::create(path).expect("create failed");
        f.write_all(content).expect("write failed");
    }
    println!("OK create_and_write");

    // Test 2: Read back the file
    let data = std::fs::read(path).expect("read failed");
    assert_eq!(&data, content, "read mismatch");
    println!("OK read_back");

    // Test 3: Read /proc/version
    let version = std::fs::read_to_string("/proc/version").expect("read /proc/version failed");
    assert!(version.contains("Solaya"), "version mismatch");
    println!("OK proc_version");

    // Test 4: Remove file
    std::fs::remove_file(path).expect("remove failed");
    println!("OK remove");

    // Test 5: Confirm file is gone
    assert!(std::fs::read(path).is_err(), "file should be gone");
    println!("OK gone");
}
