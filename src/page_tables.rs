use core::{arch::asm, fmt::Debug, ptr::NonNull};

use crate::{
    page_allocator, println,
    uart::UART_BASE_ADDRESS,
    util::{get_bit, set_multiple_bits, set_or_clear_bit},
};

static mut CURRENT_PAGE_TABLE: Option<&'static PageTable> = None;

#[repr(transparent)]
pub struct PageTable([PageTableEntry; 512]);

impl PageTable {
    fn new() -> &'static mut PageTable {
        let page = page_allocator::zalloc(1).expect("Memory should be available.");
        let mut page_table: NonNull<PageTable> = page.addr().cast();
        unsafe { page_table.as_mut() }
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

    fn get_physical_address(&self) -> u64 {
        self as *const Self as u64
    }
}

#[repr(transparent)]
struct PageTableEntry(u64);

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum XWRMode {
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

    fn set_physical_address(&mut self, address: u64) {
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
        assert!(self.get_physical_address() != 0);
        let phyiscal_address = self.get_physical_address();
        unsafe { &mut *(phyiscal_address as *const PageTable as *mut PageTable) }
    }

    fn clear(&mut self) {
        self.0 = 0;
    }
}

pub struct MappingInformation {
    start_address: usize,
    end_address: usize,
    privileges: XWRMode,
    name: &'static str,
}

impl MappingInformation {
    pub fn new(
        start_address: usize,
        end_address: usize,
        privileges: XWRMode,
        name: &'static str,
    ) -> Self {
        assert!(end_address > start_address);
        Self {
            start_address,
            end_address,
            privileges,
            name,
        }
    }
}

impl Debug for MappingInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Mapping {}:\t0x{:x}-0x{:x} ({:?})",
            self.name, self.start_address, self.end_address, self.privileges
        )
    }
}

pub fn create_identity_mapping(mapping_information: &[MappingInformation]) -> &'static PageTable {
    let root_page_table = PageTable::new();

    for mapping in mapping_information {
        println!("{:?}", mapping);
        assert!(mapping.start_address % 4096 == 0);
        for address in (mapping.start_address..mapping.end_address).step_by(4096) {
            // println!("Map address 0x{:x}", address);
            let first_level_entry = root_page_table.get_entry_for_virtual_address(address, 2);
            if first_level_entry.get_physical_address() == 0 {
                let new_page_table = PageTable::new();
                first_level_entry.set_physical_address(new_page_table.get_physical_address());
                first_level_entry.set_validity(true);
            }

            let second_level_entry = first_level_entry
                .get_target_page_table()
                .get_entry_for_virtual_address(address, 1);
            if second_level_entry.get_physical_address() == 0 {
                let new_page_table = PageTable::new();
                second_level_entry.set_physical_address(new_page_table.get_physical_address());
                second_level_entry.set_validity(true);
            }

            let third_level_entry = second_level_entry
                .get_target_page_table()
                .get_entry_for_virtual_address(address, 0);

            third_level_entry.set_xwr_mode(mapping.privileges);
            third_level_entry.set_validity(true);
            third_level_entry.set_physical_address(address as u64);
        }
    }

    root_page_table
}

pub fn setup_kernel_identity_mapping() {
    println!("Setup page tables identity mapping");

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

    let mapping_information: &[MappingInformation] = unsafe {
        &[
            MappingInformation::new(TEXT_START, TEXT_END, XWRMode::ReadExecute, "TEXT"),
            MappingInformation::new(RODATA_START, RODATA_END, XWRMode::ReadOnly, "RODATA"),
            MappingInformation::new(DATA_START, DATA_END, XWRMode::ReadWrite, "DATA"),
            MappingInformation::new(
                HEAP_START,
                HEAP_START + HEAP_SIZE,
                XWRMode::ReadWrite,
                "HEAP",
            ),
            MappingInformation::new(
                UART_BASE_ADDRESS,
                UART_BASE_ADDRESS + 4096,
                XWRMode::ReadWrite,
                "UART",
            ),
        ]
    };

    println!("Create identitiy mapping");

    let identity_mapped_pagetable = create_identity_mapping(mapping_information);

    let old_page_table = activate_page_table(identity_mapped_pagetable);
    assert!(old_page_table.is_none());
}

fn activate_page_table(page_table: &'static PageTable) -> Option<&'static PageTable> {
    let page_table_address = page_table as *const PageTable as usize;
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

    let old_page_table = unsafe { CURRENT_PAGE_TABLE.replace(page_table) };

    println!("Done!\n");

    old_page_table
}
