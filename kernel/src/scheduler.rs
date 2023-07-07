macro_rules! prog_bytes {
    ($prog_name:literal) => {
        include_bytes!(concat!(
            "../../target/riscv64gc-unknown-none-elf/debug/",
            $prog_name
        ))
    };
}

const PROG1: &[u8] = prog_bytes!("prog1");
