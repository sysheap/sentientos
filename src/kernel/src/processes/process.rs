use core::fmt::Debug;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use common::{
    mutex::Mutex,
    syscalls::trap_frame::{Register, TrapFrame},
};

use crate::{
    debug,
    klibc::elf::ElfFile,
    memory::{page_allocator::AllocatedPages, page_tables::RootPageTableHolder},
    processes::loader::{self, LoadedElf},
};

fn get_next_pid() -> u64 {
    static PID_COUNTER: Mutex<u64> = Mutex::new(0);
    let mut pid_counter = PID_COUNTER.lock();
    let next_pid = *pid_counter;
    *pid_counter += 1;
    next_pid
}

pub struct Process {
    pid: u64,
    register_state: Box<TrapFrame>,
    page_table: Rc<RootPageTableHolder>,
    program_counter: usize,
    allocated_pages: Vec<AllocatedPages>,
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

        let LoadedElf {
            entry_address,
            page_tables,
            allocated_pages,
        } = loader::load_elf(elf_file);

        let mut register_state = TrapFrame::zero();
        register_state[Register::sp] = loader::STACK_START;

        Self {
            pid: get_next_pid(),
            register_state: Box::new(register_state),
            page_table: Rc::new(page_tables),
            program_counter: entry_address,
            allocated_pages: allocated_pages,
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        debug!(
            "Drop process (PID: {}) (Allocated pages: {:?})",
            self.pid, self.allocated_pages
        );
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
