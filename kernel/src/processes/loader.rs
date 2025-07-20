use alloc::{string::ToString, vec::Vec};
use common::errors::LoaderError;

use crate::{
    debug,
    klibc::{
        elf::{ElfFile, ProgramHeaderType},
        util::{copy_slice, minimum_amount_of_pages},
    },
    memory::{
        PAGE_SIZE,
        page::{Pages, PinnedHeapPages},
        page_tables::RootPageTableHolder,
    },
};

pub const STACK_START: usize = usize::MAX;

pub const STACK_SIZE_PAGES: usize = 4;
pub const STACK_SIZE: usize = PAGE_SIZE * STACK_SIZE_PAGES;

pub const STACK_END: usize = STACK_START - STACK_SIZE + 1;

#[derive(Debug)]
pub struct LoadedElf {
    pub entry_address: usize,
    pub page_tables: RootPageTableHolder,
    pub allocated_pages: Vec<PinnedHeapPages>,
    pub args_start: usize,
}

fn set_up_arguments(stack: &mut [u8], name: &str, args: &[&str]) -> Result<usize, LoaderError> {
    let mut total_bytes = name.len() + args.iter().map(|arg| arg.len()).sum::<usize>();
    // add zero bytes into account (name, number of args, zero-byte terminator)
    total_bytes += 1 + args.len() + 1;

    let stack_size = stack.len();

    if total_bytes >= stack_size {
        return Err(LoaderError::StackToSmall);
    }

    let mut offset = stack_size - total_bytes;

    copy_slice(name.as_bytes(), &mut stack[offset..]);
    offset += name.len() + 1;

    for arg in args {
        copy_slice(arg.as_bytes(), &mut stack[offset..]);
        offset += arg.len() + 1;
    }

    assert_eq!(
        stack[offset..].len(),
        1,
        "We should only have one byte left"
    );

    // We want to point into the arguments
    Ok(STACK_START - total_bytes + 1)
}

pub fn load_elf(elf_file: &ElfFile, name: &str, args: &[&str]) -> Result<LoadedElf, LoaderError> {
    let mut page_tables = RootPageTableHolder::new_with_kernel_mapping();

    let elf_header = elf_file.get_header();
    let mut allocated_pages = Vec::new();

    // Map 4KB stack
    let mut stack = PinnedHeapPages::new(STACK_SIZE_PAGES);

    let args_start = set_up_arguments(stack.as_u8_slice(), name, args)?;

    let stack_addr = stack.addr();
    allocated_pages.push(stack);

    debug!(
        "before mapping stack: stack_start={STACK_START:#x} stack_size={STACK_SIZE:#x} stack_end={STACK_END:#x}"
    );

    page_tables.map_userspace(
        STACK_END,
        stack_addr.get(),
        STACK_SIZE,
        crate::memory::page_tables::XWRMode::ReadWrite,
        "Stack".to_string(),
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
            "LOAD".to_string(),
        );
    }

    Ok(LoadedElf {
        entry_address: elf_header.entry_point as usize,
        page_tables,
        allocated_pages,
        args_start,
    })
}
