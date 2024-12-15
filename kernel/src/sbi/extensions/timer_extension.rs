use crate::sbi::{self, sbi_call::SbiRet};

pub const EID: u64 = 0x54494D45;
pub const FID_SET_TIMER: u64 = 0x0;

pub fn sbi_set_timer(stime_value: u64) -> SbiRet {
    sbi::sbi_call_1(EID, FID_SET_TIMER, stime_value)
}
