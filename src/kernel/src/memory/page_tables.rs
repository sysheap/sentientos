use core::{arch::asm, fmt::Debug, ptr::NonNull, u8};

use alloc::rc::Rc;
use common::mutex::Mutex;

use crate::{
    debug,
    interrupts::plic,
    klibc::{
        elf,
        util::{get_bit, get_multiple_bits, set_multiple_bits, set_or_clear_bit},
    },
    memory::page_allocator::PAGE_SIZE,
    processes::timer,
    test::qemu_exit,
};

use super::page_allocator::{self, PagePointer};

static CURRENT_PAGE_TABLE: Mutex<Option<Rc<RootPageTableHolder>>> = Mutex::new(None);

pub struct RootPageTableHolder(Mutex<&'static mut PageTable>);

impl Debug for RootPageTableHolder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let page_table = self.0.lock();
        write!(f, "RootPageTableHolder({:p})", &*page_table)
    }
}

impl RootPageTableHolder {
    fn empty() -> Self {
        Self(Mutex::new(PageTable::new()))
    }

    pub fn new_with_kernel_mapping() -> Self {
        let root_page_table_holder = RootPageTableHolder::empty();

        extern "C" {
            static mut TEXT_START: usize;
            static mut TEXT_END: usize;
            static mut RODATA_START: usize;
            static mut RODATA_END: usize;
            static mut DATA_START: usize;
            static mut DATA_END: usize;

            static mut HEAP_START: usize;
            static mut HEAP_SIZE: usize;
        }

        unsafe {
            root_page_table_holder.map_identity_kernel(
                TEXT_START,
                TEXT_END - TEXT_START,
                XWRMode::ReadExecute,
                "TEXT",
            );

            root_page_table_holder.map_identity_kernel(
                RODATA_START,
                RODATA_END - RODATA_START,
                XWRMode::ReadOnly,
                "RODATA",
            );

            root_page_table_holder.map_identity_kernel(
                DATA_START,
                DATA_END - DATA_START,
                XWRMode::ReadWrite,
                "DATA",
            );

            root_page_table_holder.map_identity_kernel(
                HEAP_START,
                HEAP_SIZE,
                XWRMode::ReadWrite,
                "HEAP",
            );

            root_page_table_holder.map_identity_kernel(
                plic::PLIC_BASE,
                plic::PLIC_SIZE,
                XWRMode::ReadWrite,
                "PLIC",
            );

            root_page_table_holder.map_identity_kernel(
                timer::CLINT_BASE,
                timer::CLINT_SIZE,
                XWRMode::ReadWrite,
                "CLINT",
            );

            root_page_table_holder.map_identity_kernel(
                qemu_exit::TEST_DEVICE_ADDRESSS,
                PAGE_SIZE,
                XWRMode::ReadWrite,
                "Qemu Test Device",
            );
        }

        root_page_table_holder
    }

    pub fn map_kernel(
        &self,
        virtual_address_start: usize,
        physical_address_start: usize,
        size: usize,
        privileges: XWRMode,
        name: &str,
    ) {
        self.map(
            virtual_address_start,
            physical_address_start,
            size,
            privileges,
            false,
            name,
        );
    }

    pub fn map_userspace(
        &self,
        virtual_address_start: usize,
        physical_address_start: usize,
        size: usize,
        privileges: XWRMode,
        name: &str,
    ) {
        self.map(
            virtual_address_start,
            physical_address_start,
            size,
            privileges,
            true,
            name,
        );
    }

    fn get_page_table_entry_for_address(&self, address: usize) -> Option<&PageTableEntry> {
        let root_page_table = self.0.lock();

        let first_level_entry = root_page_table.get_entry_for_virtual_address(address, 2);
        if !first_level_entry.get_validity() {
            return None;
        }

        let second_level_entry = first_level_entry
            .get_target_page_table()
            .get_entry_for_virtual_address(address, 1);
        if !second_level_entry.get_validity() {
            return None;
        }

        let third_level_entry = second_level_entry
            .get_target_page_table()
            .get_entry_for_virtual_address(address, 0);
        if !third_level_entry.get_validity() {
            return None;
        }

        Some(third_level_entry)
    }

    fn map(
        &self,
        virtual_address_start: usize,
        physical_address_start: usize,
        size: usize,
        privileges: XWRMode,
        is_user_mode_accessible: bool,
        name: &str,
    ) {
        debug!(
            "Map \t{:#018x}-{:#018x} -> {:#018x}-{:#018x} (Size: {:#010x}) ({:?})\t({})",
            virtual_address_start,
            virtual_address_start - PAGE_SIZE + size,
            physical_address_start,
            physical_address_start - PAGE_SIZE + size,
            size,
            privileges,
            name
        );

        assert_eq!(virtual_address_start % PAGE_SIZE, 0);
        assert_eq!(physical_address_start % PAGE_SIZE, 0);
        assert_eq!(size % PAGE_SIZE, 0);

        let mut root_page_table = self.0.lock();

        for offset in (0..size).step_by(PAGE_SIZE) {
            let current_virtual_address = virtual_address_start + offset;
            let current_physical_address = physical_address_start + offset;

            let first_level_entry =
                root_page_table.get_entry_for_virtual_address_mut(current_virtual_address, 2);
            if first_level_entry.get_physical_address() == 0 {
                let new_page_table = PageTable::new();
                first_level_entry.set_physical_address(new_page_table.get_physical_address());
                first_level_entry.set_validity(true);
            }

            let second_level_entry = first_level_entry
                .get_target_page_table()
                .get_entry_for_virtual_address_mut(current_virtual_address, 1);
            if second_level_entry.get_physical_address() == 0 {
                let new_page_table = PageTable::new();
                second_level_entry.set_physical_address(new_page_table.get_physical_address());
                second_level_entry.set_validity(true);
            }

            let third_level_entry = second_level_entry
                .get_target_page_table()
                .get_entry_for_virtual_address_mut(current_virtual_address, 0);

            assert!(!third_level_entry.get_validity());

            third_level_entry.set_xwr_mode(privileges);
            third_level_entry.set_validity(true);
            third_level_entry.set_physical_address(current_physical_address);
            third_level_entry.set_user_mode_accessible(is_user_mode_accessible);
        }
    }

    pub fn map_identity_kernel(
        &self,
        virtual_address_start: usize,
        size: usize,
        privileges: XWRMode,
        name: &str,
    ) {
        self.map_identity(virtual_address_start, size, privileges, false, name);
    }

    fn map_identity(
        &self,
        virtual_address_start: usize,
        size: usize,
        privileges: XWRMode,
        is_user_mode_accessible: bool,
        name: &str,
    ) {
        self.map(
            virtual_address_start,
            virtual_address_start,
            size,
            privileges,
            is_user_mode_accessible,
            name,
        );
    }
}

impl Drop for RootPageTableHolder {
    fn drop(&mut self) {
        let root_page_table = self.0.lock();
        drop_recursive(&root_page_table);
        fn drop_recursive(page_table: &PageTable) {
            // Iterate through all childs
            for entry in &page_table.0 {
                if entry.get_validity() && !entry.is_leaf() {
                    drop_recursive(entry.get_target_page_table());
                }
            }

            page_allocator::dealloc(page_table.get_page_pointer());
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct PageTable([PageTableEntry; 512]);

impl PageTable {
    fn new() -> &'static mut PageTable {
        let page = page_allocator::zalloc(1).expect("Memory should be available.");
        let mut page_table: NonNull<PageTable> = page.addr().cast();
        unsafe { page_table.as_mut() }
    }

    fn get_page_pointer(&self) -> PagePointer {
        self.get_physical_address().into()
    }

    fn get_entry_for_virtual_address_mut(
        &mut self,
        virtual_address: usize,
        level: u8,
    ) -> &mut PageTableEntry {
        assert!(level <= 2);
        let shifted_address = virtual_address >> (12 + 9 * level);
        let index = shifted_address & 0x1ff;
        &mut self.0[index]
    }

    fn get_entry_for_virtual_address(&self, virtual_address: usize, level: u8) -> &PageTableEntry {
        assert!(level <= 2);
        let shifted_address = virtual_address >> (12 + 9 * level);
        let index = shifted_address & 0x1ff;
        &self.0[index]
    }

    fn get_physical_address(&self) -> usize {
        self as *const Self as usize
    }
}

#[repr(transparent)]
#[derive(Debug)]
struct PageTableEntry(u64);

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
        unsafe { core::mem::transmute(value) }
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

impl PageTableEntry {
    const VALID_BIT_POS: usize = 0;
    const READ_BIT_POS: usize = 1;
    const WRITE_BIT_POS: usize = 2;
    const EXECUTE_BIT_POS: usize = 3;
    const USER_MODE_ACCESSIBLE_BIT_POS: usize = 4;
    const PHYSICAL_PAGE_BIT_POS: usize = 10;

    fn set_validity(&mut self, is_valid: bool) {
        set_or_clear_bit(&mut self.0, is_valid, PageTableEntry::VALID_BIT_POS);
    }

    fn get_validity(&self) -> bool {
        get_bit(self.0, PageTableEntry::VALID_BIT_POS)
    }

    fn set_user_mode_accessible(&mut self, is_user_mode_accessible: bool) {
        set_or_clear_bit(
            &mut self.0,
            is_user_mode_accessible,
            PageTableEntry::USER_MODE_ACCESSIBLE_BIT_POS,
        );
    }

    fn get_user_mode_accessible(&self) -> bool {
        get_bit(self.0, PageTableEntry::USER_MODE_ACCESSIBLE_BIT_POS)
    }

    fn set_xwr_mode(&mut self, mode: XWRMode) {
        set_multiple_bits(&mut self.0, mode as u8, 3, PageTableEntry::READ_BIT_POS);
    }

    fn get_xwr_mode(&self) -> XWRMode {
        let bits = get_multiple_bits(self.0, 3, PageTableEntry::READ_BIT_POS) as u8;
        bits.into()
    }

    fn is_leaf(&self) -> bool {
        let mode = self.get_xwr_mode();
        mode != XWRMode::PointerToNextLevel
    }

    fn set_physical_address(&mut self, address: usize) {
        set_multiple_bits(
            &mut self.0,
            address >> 12,
            44,
            PageTableEntry::PHYSICAL_PAGE_BIT_POS,
        );
    }

    fn get_physical_address(&self) -> u64 {
        ((self.0 >> PageTableEntry::PHYSICAL_PAGE_BIT_POS) & 0xfffffffffff) << 12
    }

    fn get_target_page_table(&self) -> &'static mut PageTable {
        assert!(!self.is_leaf());
        assert!(self.get_physical_address() != 0);
        let phyiscal_address = self.get_physical_address();
        unsafe { &mut *(phyiscal_address as *const PageTable as *mut PageTable) }
    }

    fn clear(&mut self) {
        self.0 = 0;
    }
}

pub fn activate_page_table(page_table_holder: Rc<RootPageTableHolder>) {
    let page_table_address = page_table_holder.0.lock().get_physical_address();

    debug!(
        "Activate new page mapping (Addr of page tables 0x{:x})",
        page_table_address
    );
    let page_table_address_shifted = page_table_address >> 12;

    let satp_val = 8 << 60 | (page_table_address_shifted & 0xfffffffffff);

    unsafe {
        asm!("csrw satp, {satp_val}", satp_val = in(reg) satp_val);
        asm!("sfence.vma");
    }

    CURRENT_PAGE_TABLE.lock().replace(page_table_holder);
}

pub fn is_userspace_address(address: usize) -> bool {
    let current_page_table = CURRENT_PAGE_TABLE.lock();
    if let Some(ref current_page_table) = *current_page_table {
        current_page_table
            .get_page_table_entry_for_address(address)
            .map_or(false, |entry| entry.get_user_mode_accessible())
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::RootPageTableHolder;

    #[test_case]
    fn check_drop_of_page_table_holder() {
        let page_table = RootPageTableHolder::new_with_kernel_mapping();
        drop(page_table);
    }
}
