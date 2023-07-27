use alloc::vec::Vec;

use crate::{
    interrupts::trap::TrapFrame,
    klibc::{
        elf::{ElfFile, ProgramHeaderType},
        util::{align_up_number_of_pages, copy_slice},
    },
    memory::{
        page_allocator::{dealloc, zalloc, PagePointer, PAGE_SIZE},
        page_tables::RootPageTableHolder,
    },
    println,
};

pub struct Process {
    register_state: TrapFrame,
    page_table: RootPageTableHolder,
    program_counter: usize,
    allocated_pages: Vec<PagePointer>,
    status: ProcessStatus,
}

pub enum ProcessStatus {
    Running,
    ReadyToBeScheduled,
}

impl Process {
    const STACK_START: usize = 0x7ffffffffffff000;
    const STACK_END: usize = Process::STACK_START + (PAGE_SIZE - 1);

    pub fn from_elf(elf_file: &ElfFile) -> Self {
        println!("Create process from elf file");

        let page_table = RootPageTableHolder::new_with_kernel_mapping();
        let mut register_state = TrapFrame::zero();

        let elf_header = elf_file.get_header();
        let mut allocated_pages = Vec::new();

        // Map 4KB stack
        let stack = zalloc(PAGE_SIZE).expect("Could not allocate memory for stack");
        allocated_pages.push(stack.clone());

        page_table.map_userspace(
            Process::STACK_START,
            stack.addr_as_usize(),
            PAGE_SIZE,
            crate::memory::page_tables::XWRMode::ReadWrite,
            "Stack",
        );

        register_state.set_stack_pointer(Process::STACK_START);

        // Map load program header
        let loadable_program_header = elf_file
            .get_program_headers()
            .iter()
            .filter(|header| header.header_type == ProgramHeaderType::PT_LOAD);

        for program_header in loadable_program_header {
            let data = elf_file.get_program_header_data(program_header);
            let size_in_pages = align_up_number_of_pages(data.len());
            let pages =
                zalloc(size_in_pages).expect("Could not allocate memory for program header.");
            allocated_pages.push(pages.clone());
            let page_slice = pages.slice();
            copy_slice(data, page_slice);

            page_table.map_userspace(
                program_header.virtual_address as usize,
                pages.addr_as_usize(),
                size_in_pages * PAGE_SIZE,
                program_header.access_flags.into(),
                "LOAD",
            );
        }

        println!("DONE");

        Self {
            register_state,
            page_table,
            program_counter: elf_header.entry_point as usize,
            allocated_pages,
            status: ProcessStatus::ReadyToBeScheduled,
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        for allocated_page in &self.allocated_pages {
            dealloc(allocated_page.clone());
        }
    }
}
