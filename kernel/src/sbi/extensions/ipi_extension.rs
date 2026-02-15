use crate::sbi::{self, sbi_call::SbiRet};

pub const EID: u64 = 0x735049;
pub const FID_SEND_IPI: u64 = 0x0;

pub fn sbi_send_ipi(hart_mask: u64, hart_mask_base: i64) -> SbiRet {
    sbi::sbi_call(
        EID,
        FID_SEND_IPI,
        hart_mask,
        crate::klibc::util::bitcast_i64_to_u64(hart_mask_base),
        0,
    )
}
