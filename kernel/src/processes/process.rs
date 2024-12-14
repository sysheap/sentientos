use crate::{
    debug,
    klibc::elf::ElfFile,
    memory::{page::PinnedHeapPages, page_tables::RootPageTableHolder, PAGE_SIZE},
    net::sockets::SharedAssignedSocket,
    processes::loader::{self, LoadedElf},
};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use common::{
    net::UDPDescriptor,
    syscalls::trap_frame::{Register, TrapFrame},
};
use core::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

pub type Pid = u64;

const FREE_MMAP_START_ADDRESS: usize = 0x2000000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Runnable,
    WaitingFor(Pid),
    WaitingForInput,
}

impl ProcessState {
    pub fn is_waiting(&self) -> bool {
        matches!(
            self,
            ProcessState::WaitingFor(_) | ProcessState::WaitingForInput
        )
    }
}

fn get_next_pid() -> Pid {
    static PID_COUNTER: AtomicU64 = AtomicU64::new(0);
    let next_pid = PID_COUNTER.fetch_add(1, Ordering::Relaxed);
    assert_ne!(next_pid, u64::MAX, "We ran out of process pids");
    next_pid
}

pub struct Process {
    name: String,
    pid: Pid,
    register_state: TrapFrame,
    page_table: RootPageTableHolder,
    program_counter: usize,
    allocated_pages: Vec<PinnedHeapPages>,
    state: ProcessState,
    free_mmap_address: usize,
    next_free_descriptor: u64,
    open_udp_sockets: BTreeMap<UDPDescriptor, SharedAssignedSocket>,
    in_kernel_mode: bool,
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
            Number of allocated pages: {},
            State: {:?},
            In kernel mode: {}
        ]",
            self.pid,
            self.register_state,
            self.page_table,
            self.program_counter,
            self.allocated_pages.len(),
            self.state,
            self.in_kernel_mode
        )
    }
}

impl Process {
    pub fn mmap_pages(&mut self, number_of_pages: usize) -> *mut u8 {
        let pages = PinnedHeapPages::new(number_of_pages);
        self.page_table.map_userspace(
            self.free_mmap_address,
            pages.as_ptr() as usize,
            PAGE_SIZE * number_of_pages,
            crate::memory::page_tables::XWRMode::ReadWrite,
            "Heap",
        );
        self.allocated_pages.push(pages);
        let ptr = self.free_mmap_address as *mut u8;
        self.free_mmap_address += number_of_pages * PAGE_SIZE;
        ptr
    }

    pub fn get_register_state(&self) -> &TrapFrame {
        &self.register_state
    }

    pub fn set_register_state(&mut self, register_state: &TrapFrame) {
        self.register_state = *register_state;
    }

    pub fn get_program_counter(&self) -> usize {
        self.program_counter
    }

    pub fn set_program_counter(&mut self, program_counter: usize) {
        self.program_counter = program_counter;
    }

    pub fn get_state(&self) -> ProcessState {
        self.state
    }

    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }

    pub fn get_page_table(&self) -> &RootPageTableHolder {
        &self.page_table
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_pid(&self) -> Pid {
        self.pid
    }

    pub fn set_syscall_return_code(&mut self, return_code: usize) {
        self.register_state[Register::a0] = return_code;
    }

    pub fn set_in_kernel_mode(&mut self, in_kernel_mode: bool) {
        self.in_kernel_mode = in_kernel_mode;
    }

    pub fn get_in_kernel_mode(&self) -> bool {
        self.in_kernel_mode
    }

    pub fn from_elf(elf_file: &ElfFile, name: &str) -> Self {
        debug!("Create process from elf file");

        let LoadedElf {
            entry_address,
            page_tables: page_table,
            allocated_pages,
        } = loader::load_elf(elf_file);

        let mut register_state = TrapFrame::zero();
        register_state[Register::sp] = loader::STACK_START;

        Self {
            name: name.into(),
            pid: get_next_pid(),
            register_state,
            page_table,
            program_counter: entry_address,
            allocated_pages,
            state: ProcessState::Runnable,
            free_mmap_address: FREE_MMAP_START_ADDRESS,
            next_free_descriptor: 0,
            open_udp_sockets: BTreeMap::new(),
            in_kernel_mode: false,
        }
    }

    pub fn put_new_udp_socket(&mut self, socket: SharedAssignedSocket) -> UDPDescriptor {
        let descriptor = UDPDescriptor::new(self.next_free_descriptor);
        self.next_free_descriptor += 1;

        assert!(
            self.open_udp_sockets.insert(descriptor, socket).is_none(),
            "Descriptor must be empty."
        );

        descriptor
    }

    pub fn get_shared_udp_socket(
        &mut self,
        descriptor: UDPDescriptor,
    ) -> Option<&mut SharedAssignedSocket> {
        self.open_udp_sockets.get_mut(&descriptor)
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
    use crate::{
        autogenerated::userspace_programs::PROG1, klibc::elf::ElfFile, memory::PAGE_SIZE,
        processes::process::FREE_MMAP_START_ADDRESS,
    };

    use super::Process;

    #[test_case]
    fn create_process_from_elf() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let _process = Process::from_elf(&elf, "prog1");
    }

    #[test_case]
    fn mmap_process() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let mut process = Process::from_elf(&elf, "prog1");
        assert!(
            process.free_mmap_address == FREE_MMAP_START_ADDRESS,
            "Free MMAP Address must set to correct start"
        );
        let ptr = process.mmap_pages(1);
        assert!(
            ptr as usize == FREE_MMAP_START_ADDRESS,
            "Returned pointer must have the value of the initial free mmap start address."
        );
        assert!(
            process.free_mmap_address == FREE_MMAP_START_ADDRESS + PAGE_SIZE,
            "Free mmap address must have the value of the next free value"
        );
        let ptr = process.mmap_pages(2);
        assert!(
            ptr as usize == FREE_MMAP_START_ADDRESS + PAGE_SIZE,
            "Returned pointer must have the value of the initial free mmap start address."
        );
        assert!(
            process.free_mmap_address == FREE_MMAP_START_ADDRESS + (3 * PAGE_SIZE),
            "Free mmap address must have the value of the next free value"
        );
    }
}
