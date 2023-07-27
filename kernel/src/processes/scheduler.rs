macro_rules! prog_bytes {
    ($prog_ident:ident, $prog_name:literal) => {
        const $prog_ident: &[u8] = include_bytes!(concat!(
            "../../../target/riscv64gc-unknown-none-elf/debug/",
            $prog_name
        ));
    };
}

prog_bytes!(PROG1, "prog1");
