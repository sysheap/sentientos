use crate::klibc::MMIO;

pub const TEST_DEVICE_ADDRESSS: usize = 0x100000;

const EXIT_SUCCESS_CODE: u32 = 0x5555;
const EXIT_FAILURE_CODE: u32 = 0x3333;
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
