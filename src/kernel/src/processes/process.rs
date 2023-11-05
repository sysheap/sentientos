use core::fmt::Debug;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use common::{
    mutex::Mutex,
    syscalls::trap_frame::{Register, TrapFrame},
};

use crate::{
    debug,
    klibc::{
        elf::{ElfFile, ProgramHeaderType},
        util::{align_up_and_get_number_of_pages, copy_slice},
    },
    memory::{
        page_allocator::{dealloc, zalloc, PagePointer, PAGE_SIZE},
        page_tables::RootPageTableHolder,
    },
};

pub struct Process {
    pid: usize,
    register_state: Box<TrapFrame>,
    page_table: Rc<RootPageTableHolder>,
    program_counter: usize,
    allocated_pages: Vec<PagePointer>,
}

impl Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Process [
            PID: {},
            Registers: {:?},
            Page Table: {:?},
            Program Counter: {:#x},
            Number of allocated pages: {}
        ]",
            self.pid,
            self.register_state,
            self.page_table,
            self.program_counter,
            self.allocated_pages.len()
        )
    }
}

impl Process {
    const STACK_END: usize = 0xfffffffffffff000;
    const STACK_START: usize = Process::STACK_END + (PAGE_SIZE - 1);

    pub fn register_state_ptr(&self) -> *const TrapFrame {
        self.register_state.as_ref() as *const TrapFrame
    }

    pub fn get_program_counter(&self) -> usize {
        self.program_counter
    }

    pub fn set_program_counter(&mut self, program_counter: usize) {
        self.program_counter = program_counter;
    }

    pub fn get_page_table(&self) -> Rc<RootPageTableHolder> {
        self.page_table.clone()
    }

    pub fn from_elf(elf_file: &ElfFile) -> Self {
        debug!("Create process from elf file");

        let page_table = RootPageTableHolder::new_with_kernel_mapping();
        let mut register_state = TrapFrame::zero();

        let elf_header = elf_file.get_header();
        let mut allocated_pages = Vec::new();

        // Map 4KB stack
        let stack = zalloc(PAGE_SIZE).expect("Could not allocate memory for stack");
        allocated_pages.push(stack.clone());

        page_table.map_userspace(
            Process::STACK_END,
            stack.addr_as_usize(),
            PAGE_SIZE,
            crate::memory::page_tables::XWRMode::ReadWrite,
            "Stack",
        );

        register_state[Register::sp] = Process::STACK_START;

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

            page_table.map_userspace(
                program_header.virtual_address as usize,
                pages.addr_as_usize(),
                size_in_pages * PAGE_SIZE,
                program_header.access_flags.into(),
                "LOAD",
            );
        }

        debug!("DONE (Entry: {:#x})", elf_header.entry_point);

        static PID_COUNTER: Mutex<usize> = Mutex::new(0);
        let mut pid_counter = PID_COUNTER.lock();
        let next_pid = *pid_counter;
        *pid_counter += 1;

        Self {
            pid: next_pid,
            register_state: Box::new(register_state),
            page_table: Rc::new(page_table),
            program_counter: elf_header.entry_point as usize,
            allocated_pages,
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        debug!(
            "Drop process (PID: {}) (Allocated pages: {:?})",
            self.pid, self.allocated_pages
        );
        for allocated_page in &self.allocated_pages {
            dealloc(allocated_page.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{klibc::elf::ElfFile, processes::scheduler};

    use super::Process;

    #[test_case]
    fn create_process_from_elf() {
        let elf = ElfFile::parse(scheduler::PROG1.1).expect("Cannot parse elf file");
        let process = Process::from_elf(&elf);
        drop(process);
    }
}
