use crate::klibc::{
    elf,
    util::{get_bit, get_multiple_bits, set_multiple_bits, set_or_clear_bit},
};

use super::{address::PhysAddr, page_tables::PageTable};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XWRMode {
    PointerToNextLevel = 0b000,
    ReadOnly = 0b001,
    ReadWrite = 0b011,
    ExecuteOnly = 0b100,
    ReadExecute = 0b101,
    ReadWriteExecute = 0b111,
}

impl From<u8> for XWRMode {
    fn from(value: u8) -> Self {
        match value {
            0b000 => Self::PointerToNextLevel,
            0b001 => Self::ReadOnly,
            0b011 => Self::ReadWrite,
            0b100 => Self::ExecuteOnly,
            0b101 => Self::ReadExecute,
            0b111 => Self::ReadWriteExecute,
            _ => panic!("Invalid XWR mode: {value:#05b}"),
        }
    }
}

impl From<elf::ProgramHeaderFlags> for XWRMode {
    fn from(value: elf::ProgramHeaderFlags) -> Self {
        match value {
            elf::ProgramHeaderFlags::RW => Self::ReadWrite,
            elf::ProgramHeaderFlags::RWX => Self::ReadWriteExecute,
            elf::ProgramHeaderFlags::RX => Self::ReadExecute,
            elf::ProgramHeaderFlags::X => Self::ExecuteOnly,
            elf::ProgramHeaderFlags::W => panic!("Cannot map W flag"),
            elf::ProgramHeaderFlags::WX => panic!("Cannot map WX flag"),
            elf::ProgramHeaderFlags::R => Self::ReadOnly,
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub(super) struct PageTableEntry(pub(super) *mut PageTable);

impl PageTableEntry {
    const VALID_BIT_POS: usize = 0;
    const READ_BIT_POS: usize = 1;
    #[allow(dead_code)]
    const WRITE_BIT_POS: usize = 2;
    #[allow(dead_code)]
    const EXECUTE_BIT_POS: usize = 3;
    const USER_MODE_ACCESSIBLE_BIT_POS: usize = 4;
    const PHYSICAL_PAGE_BIT_POS: usize = 10;
    const PHYSICAL_PAGE_BITS: usize = 0xfffffffffff;

    pub(super) fn set_validity(&mut self, is_valid: bool) {
        self.0 = self.0.map_addr(|mut addr| {
            set_or_clear_bit(&mut addr, is_valid, PageTableEntry::VALID_BIT_POS)
        });
    }

    pub(super) fn get_validity(&self) -> bool {
        get_bit(self.0.addr(), PageTableEntry::VALID_BIT_POS)
    }

    pub(super) fn set_user_mode_accessible(&mut self, is_user_mode_accessible: bool) {
        self.0 = self.0.map_addr(|mut addr| {
            set_or_clear_bit(
                &mut addr,
                is_user_mode_accessible,
                PageTableEntry::USER_MODE_ACCESSIBLE_BIT_POS,
            )
        });
    }

    pub(super) fn get_user_mode_accessible(&self) -> bool {
        get_bit(self.0.addr(), PageTableEntry::USER_MODE_ACCESSIBLE_BIT_POS)
    }

    pub(super) fn set_xwr_mode(&mut self, mode: XWRMode) {
        self.0 = self.0.map_addr(|mut addr| {
            set_multiple_bits(&mut addr, mode as u8, 3, PageTableEntry::READ_BIT_POS)
        });
    }

    pub(super) fn get_xwr_mode(&self) -> XWRMode {
        let bits: u8 = u8::try_from(get_multiple_bits::<u64, u64>(
            self.0.addr() as u64,
            3,
            PageTableEntry::READ_BIT_POS,
        ))
        .expect("3 bits fit in u8");
        bits.into()
    }

    pub(super) fn is_leaf(&self) -> bool {
        let mode = self.get_xwr_mode();
        mode != XWRMode::PointerToNextLevel
    }

    pub(super) fn set_physical_address(&mut self, address: *mut PageTable) {
        let mask: usize = !(Self::PHYSICAL_PAGE_BITS << Self::PHYSICAL_PAGE_BIT_POS);
        self.0 = address.map_addr(|new_address| {
            let mut original = self.0.addr();
            original &= mask;
            original |=
                ((new_address >> 12) & Self::PHYSICAL_PAGE_BITS) << Self::PHYSICAL_PAGE_BIT_POS;
            original
        });
    }

    pub(super) fn set_leaf_address(&mut self, address: PhysAddr) {
        assert!(
            address.is_page_aligned(),
            "Leaf address {} is not page-aligned",
            address
        );
        let mask: usize = !(Self::PHYSICAL_PAGE_BITS << Self::PHYSICAL_PAGE_BIT_POS);
        let address_usize = address.as_usize();
        self.0 = self.0.map_addr(|_| {
            let mut original = self.0.addr();
            original &= mask;
            original |=
                ((address_usize >> 12) & Self::PHYSICAL_PAGE_BITS) << Self::PHYSICAL_PAGE_BIT_POS;
            original
        });
    }

    pub(super) fn get_physical_address(&self) -> PhysAddr {
        let ptr = self.0.map_addr(|addr| {
            ((addr >> Self::PHYSICAL_PAGE_BIT_POS) & Self::PHYSICAL_PAGE_BITS) << 12
        });
        PhysAddr::new(ptr.addr())
    }

    pub(super) fn get_target_page_table(&self) -> *mut PageTable {
        assert!(!self.is_leaf());
        let addr = self.get_physical_address();
        assert!(addr != PhysAddr::zero());
        self.0.map_addr(|_| addr.as_usize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klibc::elf::ProgramHeaderFlags;
    use core::ptr::null_mut;

    #[test_case]
    fn page_table_entry_validity_bit() {
        let mut entry = PageTableEntry(null_mut());
        assert!(!entry.get_validity());
        entry.set_validity(true);
        assert!(entry.get_validity());
        entry.set_validity(false);
        assert!(!entry.get_validity());
    }

    #[test_case]
    fn page_table_entry_xwr_modes() {
        let modes = [
            XWRMode::PointerToNextLevel,
            XWRMode::ReadOnly,
            XWRMode::ReadWrite,
            XWRMode::ExecuteOnly,
            XWRMode::ReadExecute,
            XWRMode::ReadWriteExecute,
        ];
        for mode in modes {
            let mut entry = PageTableEntry(null_mut());
            entry.set_xwr_mode(mode);
            assert_eq!(entry.get_xwr_mode(), mode);
        }
    }

    #[test_case]
    fn page_table_entry_user_mode_bit() {
        let mut entry = PageTableEntry(null_mut());
        assert!(!entry.get_user_mode_accessible());
        entry.set_user_mode_accessible(true);
        assert!(entry.get_user_mode_accessible());
        entry.set_user_mode_accessible(false);
        assert!(!entry.get_user_mode_accessible());
    }

    #[test_case]
    fn page_table_entry_bits_are_independent() {
        let mut entry = PageTableEntry(null_mut());
        entry.set_validity(true);
        entry.set_xwr_mode(XWRMode::ReadWrite);
        entry.set_user_mode_accessible(true);
        assert!(entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadWrite);
        assert!(entry.get_user_mode_accessible());
    }

    #[test_case]
    fn page_table_entry_is_leaf() {
        let mut entry = PageTableEntry(null_mut());
        entry.set_xwr_mode(XWRMode::PointerToNextLevel);
        assert!(!entry.is_leaf());
        for mode in [
            XWRMode::ReadOnly,
            XWRMode::ReadWrite,
            XWRMode::ExecuteOnly,
            XWRMode::ReadExecute,
            XWRMode::ReadWriteExecute,
        ] {
            entry.set_xwr_mode(mode);
            assert!(entry.is_leaf());
        }
    }

    #[test_case]
    fn page_table_entry_leaf_address_roundtrip() {
        let mut entry = PageTableEntry(null_mut());
        let addr = PhysAddr::new(0x8020_0000);
        entry.set_leaf_address(addr);
        let got = entry.get_physical_address();
        assert_eq!(got, addr);
    }

    #[test_case]
    fn page_table_entry_leaf_address_preserves_low_bits() {
        let mut entry = PageTableEntry(null_mut());
        entry.set_validity(true);
        entry.set_xwr_mode(XWRMode::ReadWrite);
        entry.set_user_mode_accessible(true);
        entry.set_leaf_address(PhysAddr::new(0x8020_0000));
        assert!(entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadWrite);
        assert!(entry.get_user_mode_accessible());
    }

    #[test_case]
    fn xwr_mode_from_program_header_flags() {
        assert_eq!(XWRMode::from(ProgramHeaderFlags::R), XWRMode::ReadOnly);
        assert_eq!(XWRMode::from(ProgramHeaderFlags::RW), XWRMode::ReadWrite);
        assert_eq!(XWRMode::from(ProgramHeaderFlags::RX), XWRMode::ReadExecute);
        assert_eq!(XWRMode::from(ProgramHeaderFlags::X), XWRMode::ExecuteOnly);
        assert_eq!(
            XWRMode::from(ProgramHeaderFlags::RWX),
            XWRMode::ReadWriteExecute
        );
    }
}
