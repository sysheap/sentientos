use core::fmt;

pub mod configuration;

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
        if $crate::logging::configuration::should_log_module(module_path!()) {
            $crate::println!("[debug][{}] {}", module_path!(), format_args!($($arg)*));
        }
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
    #[cfg(miri)]
    {
        use std::io::Write;
        std::io::stdout().lock().write_fmt(args).unwrap();
    }

    #[cfg(not(miri))]
    {
        use crate::io::uart;
        use core::fmt::Write;
        // uart::QEMU_UART.lock().write_fmt(args).unwrap();
        unsafe {
            uart::QEMU_UART.write_fmt(args).unwrap();
        }
    }
}
