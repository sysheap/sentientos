use core::fmt::Write;

use crate::klibc::MMIO;

pub const UART_BASE_ADDRESS: usize = 0x1000_0000;

pub static mut QEMU_UART: Uart = unsafe { Uart::new(UART_BASE_ADDRESS) };

unsafe impl Sync for Uart {}
unsafe impl Send for Uart {}

pub struct Uart {
    transmitter: MMIO<u8>,
    lcr: MMIO<u8>,
}

impl Uart {
    const unsafe fn new(uart_base_address: usize) -> Self {
        Self {
            transmitter: MMIO::new(uart_base_address),
            lcr: MMIO::new(uart_base_address + 5),
        }
    }

    pub fn init(&self) {
        let mut lcr: MMIO<u8> = unsafe { MMIO::new(UART_BASE_ADDRESS + 3) };
        let mut fifo: MMIO<u8> = unsafe { MMIO::new(UART_BASE_ADDRESS + 2) };
        let mut ier: MMIO<u8> = unsafe { MMIO::new(UART_BASE_ADDRESS + 1) };
        let lcr_value = 0b11;
        // Set word length to 8 bit
        *lcr = lcr_value;
        // Enable fifo
        *fifo = 0b1;
        // Enable receiver buffer interrupts
        *ier = 0b1;

        // If we cared about the divisor, the code below would set the divisor
        // from a global clock rate of 22.729 MHz (22,729,000 cycles per second)
        // to a signaling rate of 2400 (BAUD). We usually have much faster signalling
        // rates nowadays, but this demonstrates what the divisor actually does.
        // The formula given in the NS16500A specification for calculating the divisor
        // is:
        // divisor = ceil( (clock_hz) / (baud_sps x 16) )
        // So, we substitute our values and get:
        // divisor = ceil( 22_729_000 / (2400 x 16) )
        // divisor = ceil( 22_729_000 / 38_400 )
        // divisor = ceil( 591.901 ) = 592

        // The divisor register is two bytes (16 bits), so we need to split the value
        // 592 into two bytes. Typically, we would calculate this based on measuring
        // the clock rate, but again, for our purposes [qemu], this doesn't really do
        // anything.
        let divisor: u16 = 592;
        let divisor_least: u8 = (divisor & 0xff) as u8;
        let divisor_most: u8 = (divisor >> 8) as u8;

        // Notice that the divisor register DLL (divisor latch least) and DLM (divisor
        // latch most) have the same base address as the receiver/transmitter and the
        // interrupt enable register. To change what the base address points to, we
        // open the "divisor latch" by writing 1 into the Divisor Latch Access Bit
        // (DLAB), which is bit index 7 of the Line Control Register (LCR) which
        // is at base_address + 3.
        unsafe {
            *lcr = lcr_value | 1 << 7;

            let mut dll: MMIO<u8> = MMIO::new(UART_BASE_ADDRESS);
            let mut dlm: MMIO<u8> = MMIO::new(UART_BASE_ADDRESS + 1);

            // Now, base addresses 0 and 1 point to DLL and DLM, respectively.
            // Put the lower 8 bits of the divisor into DLL
            *dll = divisor_least;
            *dlm = divisor_most;

            // Now that we've written the divisor, we never have to touch this again. In
            // hardware, this will divide the global clock (22.729 MHz) into one suitable
            // for 2,400 signals per second. So, to once again get access to the
            // RBR/THR/IER registers, we need to close the DLAB bit by clearing it to 0.
            *lcr = lcr_value;
        }
    }

    fn write(&mut self, character: u8) {
        *self.transmitter = character
    }

    fn read(&self) -> Option<u8> {
        if *self.lcr & 1 == 0 {
            return None;
        }
        Some(*self.transmitter)
    }
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
    unsafe { QEMU_UART.read() }
}
