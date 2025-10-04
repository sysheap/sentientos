use common::mutex::Mutex;

use crate::{cpu::Cpu, io::TEST_DEVICE_ADDRESS, klibc::MMIO};

const EXIT_SUCCESS_CODE: u32 = 0x5555;
#[allow(dead_code)]
const EXIT_FAILURE_CODE: u32 = 0x3333;
#[allow(dead_code)]
const EXIT_RESET_CODE: u32 = 0x7777;

static TEST_DEVICE: Mutex<MMIO<u32>> = Mutex::new(MMIO::new(TEST_DEVICE_ADDRESS));

pub fn exit_success() -> ! {
    TEST_DEVICE.lock().write(EXIT_SUCCESS_CODE);
    wait_for_the_end();
}

#[allow(dead_code)]
pub fn exit_failure(code: u16) -> ! {
    TEST_DEVICE
        .lock()
        .write(EXIT_FAILURE_CODE | ((code as u32) << 16));
    wait_for_the_end();
}

#[allow(dead_code)]
pub fn exit_reset() -> ! {
    TEST_DEVICE.lock().write(EXIT_RESET_CODE);
    wait_for_the_end();
}

pub fn wait_for_the_end() -> ! {
    unsafe {
        Cpu::disable_global_interrupts();
    }
    loop {
        Cpu::wait_for_interrupt();
    }
}
