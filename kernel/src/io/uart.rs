use core::fmt::Write;

use crate::{klibc::Mutex, sbi};

pub static SBI_UART: Mutex<Uart> = Mutex::new(Uart::new());

pub struct Uart;

impl Uart {
    const fn new() -> Self {
        Uart {}
    }

    fn write(&self, character: u8) {
        sbi::extensions::legacy_extension::sbi_console_putchar(character).assert_success();
    }

    // fn read(&self) -> Option<u8> {
    //     unsafe {
    //         if self.lcr.read() & 1 == 0 {
    //             return None;
    //         }
    //         Some(self.transmitter.read())
    //     }
    // }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            self.write(c);
        }
        Ok(())
    }
}

pub fn read() -> Option<u8> {
    todo!("Not implemented yet");
}
