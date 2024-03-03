use common::mutex::Mutex;

use crate::{assert::assert_unreachable, io::TEST_DEVICE_ADDRESSS, klibc::MMIO};

const EXIT_SUCCESS_CODE: u32 = 0x5555;
#[allow(dead_code)]
const EXIT_FAILURE_CODE: u32 = 0x3333;
#[allow(dead_code)]
const EXIT_RESET_CODE: u32 = 0x7777;

static TEST_DEVICE: Mutex<MMIO<u32>> = Mutex::new(unsafe { MMIO::new(TEST_DEVICE_ADDRESSS) });

pub fn exit_success() -> ! {
    **TEST_DEVICE.lock() = EXIT_SUCCESS_CODE;
    assert_unreachable("QEMU Exit devicd not working");
}

#[allow(dead_code)]
pub fn exit_failure(code: u16) -> ! {
    **TEST_DEVICE.lock() = EXIT_FAILURE_CODE | ((code as u32) << 16);
    assert_unreachable("QEMU Exit devicd not working");
}

#[allow(dead_code)]
pub fn exit_reset() -> ! {
    **TEST_DEVICE.lock() = EXIT_RESET_CODE;
    assert_unreachable("QEMU Exit devicd not working");
}
