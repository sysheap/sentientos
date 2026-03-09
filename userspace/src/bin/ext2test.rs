fn main() {
    // Test 1: Read a file from the ext2 mount
    match std::fs::read_to_string("/mnt/hello.txt") {
        Ok(content) => {
            print!("FILE:{}", content);
            println!("OK read_file");
        }
        Err(e) => println!("ERR read_file: {e}"),
    }

    // Test 2: List directory entries
    match std::fs::read_dir("/mnt") {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(e) => println!("DIR:{}", e.file_name().to_string_lossy()),
                    Err(e) => println!("ERR entry: {e}"),
                }
            }
            println!("OK readdir");
        }
        Err(e) => println!("ERR readdir: {e}"),
    }

    // Test 3: Read a nested file
    match std::fs::read_to_string("/mnt/subdir/nested.txt") {
        Ok(content) => {
            print!("NESTED:{}", content);
            println!("OK nested_read");
        }
        Err(e) => println!("ERR nested_read: {e}"),
    }

    // Test 4: Attempt to write (should fail with EROFS)
    match std::fs::write("/mnt/hello.txt", b"nope") {
        Ok(()) => println!("ERR write_should_fail"),
        Err(_) => println!("OK write_erofs"),
    }

    // Test 5: Attempt to create file (should fail with EROFS)
    match std::fs::File::create("/mnt/newfile.txt") {
        Ok(_) => println!("ERR create_should_fail"),
        Err(_) => println!("OK create_erofs"),
    }
}
