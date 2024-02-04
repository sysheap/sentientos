use core::fmt::{self, Write};

use common::{
    mutex::Mutex,
    syscalls::{sys_write_char, SYSCALL_SUCCESS},
};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    WRITER.lock().write_fmt(args).unwrap();
}

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if sys_write_char(c) != SYSCALL_SUCCESS {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}

static WRITER: Mutex<Writer> = Mutex::new(Writer);
