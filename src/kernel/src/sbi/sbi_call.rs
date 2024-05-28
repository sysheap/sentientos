use core::arch::asm;

#[repr(i64)]
#[derive(Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum SbiError {
    SBI_SUCCESS = 0,
    SBI_ERR_FAILED = -1,
    SBI_ERR_NOT_SUPPORTED = -2,
    SBI_ERR_INVALID_PARAM = -3,
    SBI_ERR_DENIED = -4,
    SBI_ERR_INVALID_ADDRESS = -5,
    SBI_ERR_ALREADY_AVAILABLE = -6,
    SBI_ERR_ALREADY_STARTED = -7,
    SBI_ERR_ALREADY_STOPPED = -8,
    SBI_ERR_NO_SHMEM = -9,
}

#[must_use]
#[derive(Debug)]
pub struct SbiRet {
    pub error: SbiError,
    pub value: i64,
}

impl SbiRet {
    unsafe fn new(error: i64, value: i64) -> Self {
        Self {
            error: core::mem::transmute::<i64, SbiError>(error),
            value,
        }
    }

    pub fn assert_success(&self) {
        assert!(
            self.error == SbiError::SBI_SUCCESS,
            "SBI call failed: {:?}",
            self
        );
    }
}

impl Default for SbiRet {
    fn default() -> Self {
        Self {
            error: SbiError::SBI_SUCCESS,
            value: Default::default(),
        }
    }
}

pub fn sbi_call(eid: u64, fid: u64) -> SbiRet {
    let mut error: i64;
    let mut value: i64;

    unsafe {
        asm!("ecall", in("a7") eid, in("a6") fid, lateout("a0") error, lateout("a1") value);
        SbiRet::new(error, value)
    }
}

pub fn sbi_call_1(eid: u64, fid: u64, arg0: u64) -> SbiRet {
    let mut error: i64;
    let mut value: i64;

    unsafe {
        asm!("ecall", in("a7") eid, in("a6") fid, in("a0") arg0, lateout("a0") error, lateout("a1") value);
        SbiRet::new(error, value)
    }
}
