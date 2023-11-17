use alloc::vec::Vec;

use crate::{
    klibc::{
        elf::{ElfFile, ProgramHeaderType},
        util::{align_up_and_get_number_of_pages, copy_slice},
    },
    memory::{
        page_allocator::{zalloc, PagePointer, PAGE_SIZE},
        page_tables::RootPageTableHolder,
    },
};

pub const STACK_END: usize = 0xfffffffffffff000;
pub const STACK_START: usize = STACK_END + (PAGE_SIZE - 1);

#[derive(Debug)]
pub struct LoadedElf {
    pub entry_address: usize,
    pub page_tables: RootPageTableHolder,
    pub allocated_pages: Vec<PagePointer>,
}

pub fn load_elf(elf_file: &ElfFile) -> LoadedElf {
    let page_tables = RootPageTableHolder::new_with_kernel_mapping();

    let elf_header = elf_file.get_header();
    let mut allocated_pages = Vec::new();

    // Map 4KB stack
    let stack = zalloc(PAGE_SIZE).expect("Could not allocate memory for stack");
    allocated_pages.push(stack.clone());

    page_tables.map_userspace(
        STACK_END,
        stack.addr_as_usize(),
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
        let size_in_pages = align_up_and_get_number_of_pages(real_size as usize);
        let mut pages =
            zalloc(size_in_pages).expect("Could not allocate memory for program header.");
        allocated_pages.push(pages.clone());
        let page_slice = pages.slice();
        copy_slice(data, page_slice);

        page_tables.map_userspace(
            program_header.virtual_address as usize,
            pages.addr_as_usize(),
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
