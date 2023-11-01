use core::fmt;

use crate::io::uart;

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::println!("[info][{}] {}", module_path!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::println!("[warn][{}] {}", module_path!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::println!("[debug][{}] {}", module_path!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::logging::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    uart::QEMU_UART.lock().write_fmt(args).unwrap();
}
