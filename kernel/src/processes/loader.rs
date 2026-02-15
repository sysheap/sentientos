use alloc::{string::ToString, vec::Vec};
use common::errors::LoaderError;
use headers::syscall_types::{AT_NULL, AT_PAGESZ};

use crate::klibc::{util::align_up, writable_buffer::WritableBuffer};

use crate::{
    debug,
    klibc::{
        elf::{ElfFile, ProgramHeaderType},
        util::{self, InBytes, minimum_amount_of_pages},
    },
    memory::{
        PAGE_SIZE,
        page::{PagesAsSlice, PinnedHeapPages},
        page_tables::RootPageTableHolder,
    },
    processes::brk::Brk,
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
    pub brk: Brk,
}

fn set_up_arguments(stack: &mut [u8], name: &str, args: &[&str]) -> Result<usize, LoaderError> {
    // layout:
    // [argc, argv[0], argv[n], NULL, envp[0], envp[1], NULL, AUXV AT_NULL, NULL, name, args[0], args[1]...]
    let argc = 1 + args.len(); // name + amount of args
    let mut argv = vec![0usize; args.len() + 2]; // number of args plus name and null terminator
    let envp = [0usize];
    let auxv = [AT_PAGESZ as usize, PAGE_SIZE, AT_NULL as usize, 0];
    let strings = [name]
        .iter()
        .chain(args)
        .flat_map(|s| s.as_bytes().iter().chain(&[0]))
        .copied()
        .collect::<Vec<u8>>();

    let start_of_strings_offset =
        core::mem::size_of_val(&argc) + argv.in_bytes() + envp.in_bytes() + auxv.in_bytes();

    let total_length = align_up(start_of_strings_offset + strings.in_bytes(), 8);

    if total_length >= stack.len() {
        return Err(LoaderError::StackToSmall);
    }

    let real_start = STACK_START - total_length + 1;
    let mut addr_current_string = real_start + start_of_strings_offset;

    // Patch pointers
    argv[0] = addr_current_string;
    addr_current_string = addr_current_string.wrapping_add(name.len() + 1);
    for (idx, arg) in args.iter().enumerate() {
        argv[idx + 1] = addr_current_string;
        // It could overflow on the last element, so just use wrapping_add
        addr_current_string = addr_current_string.wrapping_add(arg.len() + 1);
    }

    let offset = stack.len() - total_length;

    let mut writable_buffer = WritableBuffer::new(&mut stack[offset..]);

    writable_buffer
        .write_usize(argc)
        .map_err(|_| LoaderError::StackToSmall)?;

    for arg in argv {
        writable_buffer
            .write_usize(arg)
            .map_err(|_| LoaderError::StackToSmall)?;
    }

    for env in envp {
        writable_buffer
            .write_usize(env)
            .map_err(|_| LoaderError::StackToSmall)?;
    }

    for aux in auxv {
        writable_buffer
            .write_usize(aux)
            .map_err(|_| LoaderError::StackToSmall)?;
    }

    writable_buffer
        .write_slice(&strings)
        .map_err(|_| LoaderError::StackToSmall)?;

    // We want to point into the arguments
    Ok(STACK_START - total_length + 1)
}

pub fn load_elf(elf_file: &ElfFile, name: &str, args: &[&str]) -> Result<LoadedElf, LoaderError> {
    let mut page_tables = RootPageTableHolder::new_with_kernel_mapping(false);

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
        stack_addr,
        STACK_SIZE,
        crate::memory::page_tables::XWRMode::ReadWrite,
        "Stack".to_string(),
    );

    // Map load program header
    let loadable_program_header = || {
        elf_file
            .get_program_headers()
            .iter()
            .filter(|header| header.header_type == ProgramHeaderType::PT_LOAD)
    };

    for program_header in loadable_program_header() {
        debug!("Load {:#X?}", program_header);

        let data = elf_file.get_program_header_data(program_header);
        let real_size = program_header.memory_size;

        let real_size_usize = util::u64_as_usize(real_size);
        assert!(
            real_size_usize >= data.len(),
            "real size must always be greater than the actual data"
        );

        let offset = util::u64_as_usize(program_header.virtual_address) % PAGE_SIZE;

        let mut size_in_pages = minimum_amount_of_pages(real_size_usize);

        // Take into account when we spill into the next page
        size_in_pages += minimum_amount_of_pages(offset + real_size_usize)
            - minimum_amount_of_pages(real_size_usize);

        let mut pages = PinnedHeapPages::new(size_in_pages);

        debug!(
            "Allocated {size_in_pages} pages and fill at offset={offset:#X} with data.len={:#X}",
            data.len()
        );

        pages.fill(data, offset);

        let pages_addr = pages.addr();

        allocated_pages.push(pages);

        page_tables.map_userspace(
            util::u64_as_usize(program_header.virtual_address) - offset,
            pages_addr,
            size_in_pages * PAGE_SIZE,
            program_header.access_flags.into(),
            "LOAD".to_string(),
        );
    }

    let bss_end = loadable_program_header()
        .map(|l| l.virtual_address + l.memory_size)
        .max();

    let brk = match bss_end {
        Some(bss_end) => {
            let (pages, brk) = Brk::new(util::u64_as_usize(bss_end), &mut page_tables);
            allocated_pages.push(pages);
            brk
        }
        None => Brk::empty(),
    };

    Ok(LoadedElf {
        entry_address: util::u64_as_usize(elf_header.entry_point),
        page_tables,
        allocated_pages,
        args_start,
        brk,
    })
}
