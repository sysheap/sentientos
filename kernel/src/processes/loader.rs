use alloc::vec::Vec;

use crate::{
    klibc::{
        elf::{ElfFile, ProgramHeaderType},
        util::minimum_amount_of_pages,
    },
    memory::{page::PinnedHeapPages, page_tables::RootPageTableHolder, PAGE_SIZE},
};

pub const STACK_END: usize = 0xfffffffffffff000;
pub const STACK_START: usize = STACK_END + (PAGE_SIZE - 1);

#[derive(Debug)]
pub struct LoadedElf {
    pub entry_address: usize,
    pub page_tables: RootPageTableHolder,
    pub allocated_pages: Vec<PinnedHeapPages>,
}

pub fn load_elf(elf_file: &ElfFile) -> LoadedElf {
    let mut page_tables = RootPageTableHolder::new_with_kernel_mapping();

    let elf_header = elf_file.get_header();
    let mut allocated_pages = Vec::new();

    // Map 4KB stack
    let mut stack = PinnedHeapPages::new(1);
    let stack_addr = stack.addr();
    allocated_pages.push(stack);

    page_tables.map_userspace(
        STACK_END,
        stack_addr.get(),
        PAGE_SIZE,
        crate::memory::page_tables::XWRMode::ReadWrite,
        "Stack",
    );

    // Map load program header
    let loadable_program_header = elf_file
        .get_program_headers()
        .iter()
        .filter(|header| header.header_type == ProgramHeaderType::PT_LOAD);

    for program_header in loadable_program_header {
        let data = elf_file.get_program_header_data(program_header);
        let real_size = program_header.memory_size;
        let size_in_pages = minimum_amount_of_pages(real_size as usize);

        let mut pages = PinnedHeapPages::new(size_in_pages);
        pages.fill(data);

        let pages_addr = pages.addr();

        allocated_pages.push(pages);

        page_tables.map_userspace(
            program_header.virtual_address as usize,
            pages_addr.get(),
            size_in_pages * PAGE_SIZE,
            program_header.access_flags.into(),
            "LOAD",
        );
    }

    LoadedElf {
        entry_address: elf_header.entry_point as usize,
        page_tables,
        allocated_pages,
    }
}
