use crate::{io::TEST_DEVICE_ADDRESSS, klibc::MMIO};

const EXIT_SUCCESS_CODE: u32 = 0x5555;
const EXIT_FAILURE_CODE: u32 = 0x3333;
#[allow(dead_code)]
const EXIT_RESET_CODE: u32 = 0x7777;

const TEST_DEVICE: MMIO<u32> = MMIO::new(TEST_DEVICE_ADDRESSS);

pub fn exit_success() {
    unsafe {
        TEST_DEVICE.write(EXIT_SUCCESS_CODE);
    }
}

pub fn exit_failure(code: u16) {
    unsafe {
        TEST_DEVICE.write(EXIT_FAILURE_CODE | ((code as u32) << 16));
    }
}

#[allow(dead_code)]
pub fn exit_reset() {
    unsafe {
        TEST_DEVICE.write(EXIT_RESET_CODE);
    }
}
