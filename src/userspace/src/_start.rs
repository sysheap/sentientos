extern "C" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        main();
    }
    #[allow(clippy::empty_loop)]
    loop {}
}
