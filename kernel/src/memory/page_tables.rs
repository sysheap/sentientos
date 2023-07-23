use core::{arch::asm, fmt::Debug, ptr::NonNull, u8};

use alloc::rc::Rc;

use crate::{
    interrupts::plic,
    io::uart::UART_BASE_ADDRESS,
    klibc::{
        util::{get_bit, get_multiple_bits, set_multiple_bits, set_or_clear_bit},
        Mutex,
    },
    memory::page_allocator::PAGE_SIZE,
    println,
};

use super::page_allocator::{self, PagePointer};

static CURRENT_PAGE_TABLE: Mutex<Option<Rc<RootPageTableHolder>>> = Mutex::new(None);

pub struct RootPageTableHolder(Mutex<&'static mut PageTable>);

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
            root_page_table_holder.map_identity(
                TEXT_START,
                TEXT_END - TEXT_START,
                XWRMode::ReadExecute,
                "TEXT",
            );

            root_page_table_holder.map_identity(
                RODATA_START,
                RODATA_END - RODATA_START,
                XWRMode::ReadOnly,
                "RODATA",
            );

            root_page_table_holder.map_identity(
                DATA_START,
                DATA_END - DATA_START,
                XWRMode::ReadWrite,
                "DATA",
            );

            root_page_table_holder.map_identity(HEAP_START, HEAP_SIZE, XWRMode::ReadWrite, "HEAP");

            root_page_table_holder.map_identity(
                UART_BASE_ADDRESS,
                PAGE_SIZE,
                XWRMode::ReadWrite,
                "UART",
            );

            root_page_table_holder.map_identity(
                plic::PLIC_BASE,
                plic::PLIC_SIZE,
                XWRMode::ReadWrite,
                "PLIC",
            );
        }

        root_page_table_holder
    }

    fn map(
        &self,
        virtual_address_start: usize,
        physical_address_start: usize,
        size: usize,
        privileges: XWRMode,
        name: &str,
    ) {
        println!(
            "Map {}\t{:#010x} -> {:#010x} (Size: {:#010x}) ({:?})",
            name, virtual_address_start, physical_address_start, size, privileges
        );

        assert_eq!(virtual_address_start % PAGE_SIZE, 0);
        assert_eq!(physical_address_start % PAGE_SIZE, 0);
        assert_eq!(size % PAGE_SIZE, 0);

        let mut root_page_table = self.0.lock();

        for offset in (0..size).step_by(PAGE_SIZE) {
            let current_virtual_address = virtual_address_start + offset;
            let current_physical_address = physical_address_start + offset;

            let first_level_entry =
                root_page_table.get_entry_for_virtual_address(current_virtual_address, 2);
            if first_level_entry.get_physical_address() == 0 {
                let new_page_table = PageTable::new();
                first_level_entry.set_physical_address(new_page_table.get_physical_address());
                first_level_entry.set_validity(true);
            }

            let second_level_entry = first_level_entry
                .get_target_page_table()
                .get_entry_for_virtual_address(current_virtual_address, 1);
            if second_level_entry.get_physical_address() == 0 {
                let new_page_table = PageTable::new();
                second_level_entry.set_physical_address(new_page_table.get_physical_address());
                second_level_entry.set_validity(true);
            }

            let third_level_entry = second_level_entry
                .get_target_page_table()
                .get_entry_for_virtual_address(current_virtual_address, 0);

            third_level_entry.set_xwr_mode(privileges);
            third_level_entry.set_validity(true);
            third_level_entry.set_physical_address(current_physical_address);
        }
    }

    fn map_identity(
        &self,
        virtual_address_start: usize,
        size: usize,
        privileges: XWRMode,
        name: &str,
    ) {
        self.map(
            virtual_address_start,
            virtual_address_start,
            size,
            privileges,
            name,
        );
    }
}

impl Drop for RootPageTableHolder {
    fn drop(&mut self) {
        let mut root_page_table = self.0.lock();
        drop_recursive(&mut root_page_table);
        fn drop_recursive(page_table: &mut PageTable) {
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

    fn get_entry_for_virtual_address(
        &mut self,
        virtual_address: usize,
        level: u8,
    ) -> &mut PageTableEntry {
        assert!(level <= 2);
        let shifted_address = virtual_address >> (12 + 9 * level);
        let index = shifted_address & 0x1ff;
        &mut self.0[index]
    }

    fn get_physical_address(&self) -> usize {
        self as *const Self as usize
    }
}

#[repr(transparent)]
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
            PageTableEntry::VALID_BIT_POS,
        );
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

    println!(
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

    println!("Done!\n");
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
