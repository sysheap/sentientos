use crate::util::{set_multiple_bits, set_or_clear_bit};

#[repr(transparent)]
struct PageTable([PageTableEntry; 4096]);

#[repr(transparent)]
struct PageTableEntry(u64);

#[repr(u8)]
enum XWRMode {
    PointerToNextLevel = 0b000,
    ReadOnly = 0b001,
    ReadWrite = 0b011,
    ExecuteOnly = 0b100,
    ReadExecute = 0b101,
    ReadWriteExecute = 0b111,
}

impl PageTableEntry {
    const VALID_BIT_POS: usize = 0;
    const READ_BIT_POS: usize = 1;
    const WRITE_BIT_POS: usize = 2;
    const EXECUTE_BIT_POS: usize = 3;
    const USER_MODE_ACCESSIBLE_BIT_POS: usize = 4;

    fn set_validity(&mut self, is_valid: bool) {
        set_or_clear_bit(&mut self.0, is_valid, PageTableEntry::VALID_BIT_POS);
    }

    fn set_user_mode_accessible(&mut self, is_user_mode_accessible: bool) {
        set_or_clear_bit(
            &mut self.0,
            is_user_mode_accessible,
            PageTableEntry::VALID_BIT_POS,
        );
    }

    fn set_xwr_mode(&mut self, mode: XWRMode) {
        set_multiple_bits(&mut self.0, mode as u8, 3, PageTableEntry::READ_BIT_POS);
    }

    fn clear(&mut self) {
        self.0 = 0;
    }
}
