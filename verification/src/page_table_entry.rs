//! Model and formal proofs for RISC-V Sv39 page table entry bit manipulation.
//!
//! Mirrors `kernel/src/memory/page_table_entry.rs` and the bit utilities from
//! `kernel/src/klibc/util.rs`. The real PTE stores bits in a `*mut PageTable`
//! (repr(transparent)); all operations use `.addr()` / `.map_addr()`, so the
//! bit layout is identical to plain `usize`.

// ── Bit manipulation utilities ──
// Mirrors kernel/src/klibc/util.rs

fn get_bit(data: usize, bit_position: usize) -> bool {
    ((data >> bit_position) & 1) == 1
}

fn set_or_clear_bit(data: &mut usize, should_set: bool, bit_position: usize) {
    if should_set {
        *data |= 1usize << bit_position;
    } else {
        *data &= !(1usize << bit_position);
    }
}

/// Mirrors `set_multiple_bits` from `klibc/util.rs:192`.
fn set_multiple_bits(data: &mut usize, value: u8, number_of_bits: usize, bit_position: usize) {
    let mut mask: usize = !0;
    for idx in 0..number_of_bits {
        mask &= !(1usize << (bit_position + idx));
    }
    *data &= mask;

    mask = 0;
    for idx in 0..number_of_bits {
        if (value & (1u8 << idx)) > 0 {
            mask |= 1usize << (bit_position + idx);
        }
    }
    *data |= mask;
}

/// Mirrors `get_multiple_bits` from `klibc/util.rs:228`.
fn get_multiple_bits(data: usize, number_of_bits: usize, bit_position: usize) -> u8 {
    ((data >> bit_position) as u64 & (2u64.pow(number_of_bits as u32) - 1)) as u8
}

// ── XWR Mode enum ──
// Mirrors kernel/src/memory/page_table_entry.rs:8

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

impl XWRMode {
    fn from_bits(value: u8) -> Self {
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

    #[cfg_attr(not(kani), allow(dead_code))]
    const ALL_MODES: [XWRMode; 6] = [
        Self::PointerToNextLevel,
        Self::ReadOnly,
        Self::ReadWrite,
        Self::ExecuteOnly,
        Self::ReadExecute,
        Self::ReadWriteExecute,
    ];
}

// ── Page Table Entry Model ──
// Mirrors kernel/src/memory/page_table_entry.rs:47
//
// RISC-V Sv39 PTE layout (64 bits):
//   [0]     Valid
//   [1]     Read
//   [2]     Write
//   [3]     Execute
//   [4]     User accessible
//   [5]     Global
//   [6]     Accessed
//   [7]     Dirty
//   [9:8]   RSW (reserved)
//   [53:10] PPN (physical page number, 44 bits)
//   [63:54] Reserved

#[derive(Default)]
pub struct PageTableEntryModel {
    bits: usize,
}

impl PageTableEntryModel {
    const VALID_BIT_POS: usize = 0;
    const READ_BIT_POS: usize = 1;
    const USER_MODE_BIT_POS: usize = 4;
    const PPN_BIT_POS: usize = 10;
    const PPN_MASK: usize = 0xfffffffffff; // 44 bits

    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_validity(&mut self, is_valid: bool) {
        set_or_clear_bit(&mut self.bits, is_valid, Self::VALID_BIT_POS);
    }

    pub fn get_validity(&self) -> bool {
        get_bit(self.bits, Self::VALID_BIT_POS)
    }

    pub fn set_user_mode_accessible(&mut self, accessible: bool) {
        set_or_clear_bit(&mut self.bits, accessible, Self::USER_MODE_BIT_POS);
    }

    pub fn get_user_mode_accessible(&self) -> bool {
        get_bit(self.bits, Self::USER_MODE_BIT_POS)
    }

    pub fn set_xwr_mode(&mut self, mode: XWRMode) {
        set_multiple_bits(&mut self.bits, mode as u8, 3, Self::READ_BIT_POS);
    }

    pub fn get_xwr_mode(&self) -> XWRMode {
        let bits = get_multiple_bits(self.bits, 3, Self::READ_BIT_POS);
        XWRMode::from_bits(bits)
    }

    pub fn is_leaf(&self) -> bool {
        self.get_xwr_mode() != XWRMode::PointerToNextLevel
    }

    /// Set the PPN field. `addr` must be page-aligned (low 12 bits zero).
    /// Mirrors `set_leaf_address` in `page_table_entry.rs:118`.
    pub fn set_leaf_address(&mut self, addr: usize) {
        assert!(addr & 0xFFF == 0, "Address not page-aligned");
        let mask: usize = !(Self::PPN_MASK << Self::PPN_BIT_POS);
        self.bits &= mask;
        self.bits |= ((addr >> 12) & Self::PPN_MASK) << Self::PPN_BIT_POS;
    }

    /// Mirrors `get_physical_address` in `page_table_entry.rs:135`.
    pub fn get_physical_address(&self) -> usize {
        ((self.bits >> Self::PPN_BIT_POS) & Self::PPN_MASK) << 12
    }
}

// ── Kani proof harnesses ──

#[cfg(kani)]
mod proofs {
    use super::*;

    // ── Bit utility proofs ──

    /// Setting a bit then reading it returns the set value.
    #[kani::proof]
    fn bit_set_get_roundtrip() {
        let pos: usize = kani::any();
        kani::assume(pos < 64);
        let mut data: usize = kani::any();

        set_or_clear_bit(&mut data, true, pos);
        assert!(get_bit(data, pos));

        set_or_clear_bit(&mut data, false, pos);
        assert!(!get_bit(data, pos));
    }

    /// Setting one bit doesn't affect any other bit.
    #[kani::proof]
    fn bit_set_preserves_other_bits() {
        let pos: usize = kani::any();
        let other_pos: usize = kani::any();
        kani::assume(pos < 64 && other_pos < 64 && pos != other_pos);

        let mut data: usize = kani::any();
        let original_other = get_bit(data, other_pos);

        set_or_clear_bit(&mut data, true, pos);
        assert_eq!(get_bit(data, other_pos), original_other);
    }

    /// set_multiple_bits then get_multiple_bits roundtrips for 3-bit values.
    #[kani::proof]
    #[kani::unwind(5)]
    fn multiple_bits_roundtrip() {
        let value: u8 = kani::any();
        kani::assume(value < 8); // 3 bits max
        let pos: usize = kani::any();
        kani::assume(pos <= 60); // room for 3 bits

        let mut data: usize = 0;
        set_multiple_bits(&mut data, value, 3, pos);
        let got = get_multiple_bits(data, 3, pos);
        assert_eq!(got, value);
    }

    // ── PTE field proofs ──

    #[kani::proof]
    fn validity_roundtrip() {
        let mut entry = PageTableEntryModel::new();
        let valid: bool = kani::any();
        entry.set_validity(valid);
        assert_eq!(entry.get_validity(), valid);
    }

    /// All 6 valid XWR modes roundtrip correctly.
    #[kani::proof]
    #[kani::unwind(5)]
    fn xwr_mode_roundtrip() {
        let mut entry = PageTableEntryModel::new();
        let mode_idx: usize = kani::any();
        kani::assume(mode_idx < XWRMode::ALL_MODES.len());
        let mode = XWRMode::ALL_MODES[mode_idx];

        entry.set_xwr_mode(mode);
        assert_eq!(entry.get_xwr_mode(), mode);
    }

    #[kani::proof]
    fn user_mode_roundtrip() {
        let mut entry = PageTableEntryModel::new();
        let accessible: bool = kani::any();
        entry.set_user_mode_accessible(accessible);
        assert_eq!(entry.get_user_mode_accessible(), accessible);
    }

    /// PPN roundtrips for all valid 44-bit physical page numbers.
    #[kani::proof]
    fn address_roundtrip() {
        let mut entry = PageTableEntryModel::new();
        let ppn: usize = kani::any();
        kani::assume(ppn < (1usize << 44));
        let addr = ppn << 12;

        entry.set_leaf_address(addr);
        assert_eq!(entry.get_physical_address(), addr);
    }

    /// All four fields (validity, XWR, user-mode, address) are independent.
    #[kani::proof]
    #[kani::unwind(5)]
    fn fields_are_independent() {
        let mut entry = PageTableEntryModel::new();

        entry.set_validity(true);
        entry.set_xwr_mode(XWRMode::ReadWrite);
        entry.set_user_mode_accessible(true);
        entry.set_leaf_address(0x8020_0000);

        // All retain their values
        assert!(entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadWrite);
        assert!(entry.get_user_mode_accessible());
        assert_eq!(entry.get_physical_address(), 0x8020_0000);

        // Change XWR, others unaffected
        entry.set_xwr_mode(XWRMode::ReadExecute);
        assert!(entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadExecute);
        assert!(entry.get_user_mode_accessible());
        assert_eq!(entry.get_physical_address(), 0x8020_0000);

        // Change validity, others unaffected
        entry.set_validity(false);
        assert!(!entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadExecute);
        assert!(entry.get_user_mode_accessible());
        assert_eq!(entry.get_physical_address(), 0x8020_0000);
    }

    /// is_leaf() is true iff XWR mode != PointerToNextLevel.
    #[kani::proof]
    #[kani::unwind(5)]
    fn is_leaf_iff_not_pointer() {
        let mut entry = PageTableEntryModel::new();
        let mode_idx: usize = kani::any();
        kani::assume(mode_idx < XWRMode::ALL_MODES.len());
        let mode = XWRMode::ALL_MODES[mode_idx];

        entry.set_xwr_mode(mode);
        assert_eq!(entry.is_leaf(), mode != XWRMode::PointerToNextLevel);
    }

    /// Changing the physical address preserves all flag bits.
    #[kani::proof]
    fn address_preserves_flags() {
        let mut entry = PageTableEntryModel::new();
        entry.set_validity(true);
        entry.set_xwr_mode(XWRMode::ReadWrite);
        entry.set_user_mode_accessible(true);

        let ppn: usize = kani::any();
        kani::assume(ppn < (1usize << 44));
        entry.set_leaf_address(ppn << 12);

        assert!(entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadWrite);
        assert!(entry.get_user_mode_accessible());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_pte_operations() {
        let mut entry = PageTableEntryModel::new();
        entry.set_validity(true);
        entry.set_xwr_mode(XWRMode::ReadWrite);
        entry.set_user_mode_accessible(true);
        entry.set_leaf_address(0x8020_0000);

        assert!(entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadWrite);
        assert!(entry.get_user_mode_accessible());
        assert_eq!(entry.get_physical_address(), 0x8020_0000);
        assert!(entry.is_leaf());
    }

    #[test]
    fn pointer_is_not_leaf() {
        let mut entry = PageTableEntryModel::new();
        entry.set_xwr_mode(XWRMode::PointerToNextLevel);
        assert!(!entry.is_leaf());
    }

    #[test]
    fn all_modes_roundtrip() {
        for &mode in &XWRMode::ALL_MODES {
            let mut entry = PageTableEntryModel::new();
            entry.set_xwr_mode(mode);
            assert_eq!(entry.get_xwr_mode(), mode);
        }
    }

    #[test]
    fn address_roundtrips() {
        let addrs = [0x0, 0x1000, 0x8000_0000, 0x8020_0000, 0xFFFF_FFFF_F000];
        for addr in addrs {
            let mut entry = PageTableEntryModel::new();
            entry.set_leaf_address(addr);
            assert_eq!(entry.get_physical_address(), addr);
        }
    }

    #[test]
    fn fields_independent() {
        let mut entry = PageTableEntryModel::new();
        entry.set_validity(true);
        entry.set_xwr_mode(XWRMode::ReadExecute);
        entry.set_user_mode_accessible(true);
        entry.set_leaf_address(0x8020_0000);

        // Change address, check flags
        entry.set_leaf_address(0xDEAD_B000);
        assert!(entry.get_validity());
        assert_eq!(entry.get_xwr_mode(), XWRMode::ReadExecute);
        assert!(entry.get_user_mode_accessible());
        assert_eq!(entry.get_physical_address(), 0xDEAD_B000);
    }

    #[test]
    fn bit_utils_basic() {
        let mut val: usize = 0;
        set_or_clear_bit(&mut val, true, 3);
        assert!(get_bit(val, 3));
        assert!(!get_bit(val, 2));

        set_multiple_bits(&mut val, 0b101, 3, 8);
        assert_eq!(get_multiple_bits(val, 3, 8), 0b101);
        // bit 3 still set
        assert!(get_bit(val, 3));
    }
}
