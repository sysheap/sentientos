fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("/");
    match std::fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(e) => println!("{}", e.file_name().to_string_lossy()),
                    Err(e) => eprintln!("entry error: {e}"),
                }
            }
        }
        Err(e) => eprintln!("read_dir error: {e}"),
    }
}
