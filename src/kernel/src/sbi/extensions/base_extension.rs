use crate::sbi;

const EID: u64 = 0x10;

pub struct SbiSpecVersion {
    pub minor: u32,
    pub major: u32,
}

pub fn sbi_get_spec_version() -> SbiSpecVersion {
    let result = sbi::sbi_call(EID, 0x0);
    SbiSpecVersion {
        minor: result.value as u32 & 0xffffff,
        major: (result.value >> 24) as u32,
    }
}

pub fn sbi_probe_extension(extension_id: u64) -> bool {
    let result = sbi::sbi_call(EID, 0x3);
    result.value != 0
}
