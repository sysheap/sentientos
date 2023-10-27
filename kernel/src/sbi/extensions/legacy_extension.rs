use crate::sbi::sbi_call::{sbi_legacy_call_1, SbiRet};

pub fn sbi_console_putchar(byte: u8) -> SbiRet {
    sbi_legacy_call_1(0x1, byte as u64)
}

pub fn sbi_set_timer(stime_value: u64) -> SbiRet {
    sbi_legacy_call_1(0x0, stime_value)
}
