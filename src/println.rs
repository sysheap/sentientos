use core::fmt;

use crate::uart;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::println::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    #[allow(const_item_mutation)]
    uart::QEMU_UART.write_fmt(args).unwrap();
}
