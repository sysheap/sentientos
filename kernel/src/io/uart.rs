use core::fmt::Write;

use crate::klibc::{MMIO, Spinlock};

pub const UART_BASE_ADDRESS: usize = 0x1000_0000;

const THR_OFFSET: usize = 0;
const IER_OFFSET: usize = 1;
const FCR_OFFSET: usize = 2;
const LCR_OFFSET: usize = 3;
const LSR_OFFSET: usize = 5;
const LCR_WORD_LEN_8BIT: u8 = 0b11;
const LCR_DLAB: u8 = 1 << 7;
const FCR_ENABLE: u8 = 1;
const IER_RX_AVAILABLE: u8 = 1;
const LSR_DATA_READY: u8 = 1;
const BAUD_DIVISOR: u16 = 592;

pub static QEMU_UART: Spinlock<Uart> = Spinlock::new(Uart::new(UART_BASE_ADDRESS));

unsafe impl Sync for Uart {}
unsafe impl Send for Uart {}

pub struct Uart {
    transmitter: MMIO<u8>,
    lsr: MMIO<u8>,
    is_init: bool,
}

impl Uart {
    const fn new(uart_base_address: usize) -> Self {
        Self {
            transmitter: MMIO::new(uart_base_address + THR_OFFSET),
            lsr: MMIO::new(uart_base_address + LSR_OFFSET),
            is_init: false,
        }
    }

    pub fn init(&mut self) {
        let mut lcr: MMIO<u8> = MMIO::new(UART_BASE_ADDRESS + LCR_OFFSET);
        let mut fcr: MMIO<u8> = MMIO::new(UART_BASE_ADDRESS + FCR_OFFSET);
        let mut ier: MMIO<u8> = MMIO::new(UART_BASE_ADDRESS + IER_OFFSET);

        lcr.write(LCR_WORD_LEN_8BIT);
        fcr.write(FCR_ENABLE);
        ier.write(IER_RX_AVAILABLE);

        // Set baud rate via divisor latch.
        // divisor = ceil(22_729_000 / (2400 * 16)) = 592
        let divisor_least: u8 = (BAUD_DIVISOR & 0xff) as u8;
        let divisor_most: u8 = (BAUD_DIVISOR >> 8) as u8;

        // Open divisor latch (DLAB bit in LCR) to access DLL/DLM registers
        lcr.write(LCR_WORD_LEN_8BIT | LCR_DLAB);

        let mut dll: MMIO<u8> = MMIO::new(UART_BASE_ADDRESS + THR_OFFSET);
        let mut dlm: MMIO<u8> = MMIO::new(UART_BASE_ADDRESS + IER_OFFSET);
        dll.write(divisor_least);
        dlm.write(divisor_most);

        // Close divisor latch to restore normal register access
        lcr.write(LCR_WORD_LEN_8BIT);

        self.is_init = true;
    }

    fn write(&mut self, character: u8) {
        self.transmitter.write(character);
    }

    pub fn read(&self) -> Option<u8> {
        if self.lsr.read() & LSR_DATA_READY == 0 {
            return None;
        }
        Some(self.transmitter.read())
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if !self.is_init {
            return Ok(());
        }
        for c in s.bytes() {
            self.write(c);
        }
        Ok(())
    }
}
