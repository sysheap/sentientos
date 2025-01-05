use crate::{
    debug,
    klibc::elf::ElfFile,
    memory::{page::PinnedHeapPages, page_tables::RootPageTableHolder, PAGE_SIZE},
    net::sockets::SharedAssignedSocket,
    processes::loader::{self, LoadedElf, STACK_END},
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use common::{
    mutex::Mutex,
    net::UDPDescriptor,
    syscalls::trap_frame::{Register, TrapFrame},
};
use core::{
    any::TypeId,
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

pub type Pid = u64;

pub const POWERSAVE_PID: Pid = 0;

const FREE_MMAP_START_ADDRESS: usize = 0x2000000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Runnable,
    Waiting,
}

fn get_next_pid() -> Pid {
    // PIDs will start from 1
    // 0 is reserved for the never process which will be never scheduled
    static PID_COUNTER: AtomicU64 = AtomicU64::new(1);
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
    notify_on_die: BTreeSet<Pid>,
    waiting_on_syscall: Option<TypeId>,
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
    pub fn create_powersave_process() -> Arc<Mutex<Self>> {
        extern "C" {
            fn powersave();
        }

        let mut allocated_pages = Vec::with_capacity(1);

        // Map 4KB stack
        let mut stack = PinnedHeapPages::new(1);
        let stack_addr = stack.addr();
        allocated_pages.push(stack);

        let mut page_table = RootPageTableHolder::new_with_kernel_mapping();

        page_table.map_userspace(
            STACK_END,
            stack_addr.get(),
            PAGE_SIZE,
            crate::memory::page_tables::XWRMode::ReadWrite,
            "Stack".to_string(),
        );

        let mut register_state = TrapFrame::zero();
        register_state[Register::sp] = stack_addr.get();

        Arc::new(Mutex::new(Self {
            name: "powersave".to_string(),
            pid: POWERSAVE_PID,
            register_state,
            page_table,
            program_counter: powersave as usize,
            allocated_pages,
            state: ProcessState::Runnable,
            free_mmap_address: FREE_MMAP_START_ADDRESS,
            next_free_descriptor: 0,
            open_udp_sockets: BTreeMap::new(),
            in_kernel_mode: true,
            notify_on_die: BTreeSet::new(),
            waiting_on_syscall: None,
        }))
    }

    pub fn get_notifies_on_die(&self) -> impl Iterator<Item = &Pid> {
        self.notify_on_die.iter()
    }

    pub fn mmap_pages(&mut self, number_of_pages: usize) -> *mut u8 {
        let pages = PinnedHeapPages::new(number_of_pages);
        self.page_table.map_userspace(
            self.free_mmap_address,
            pages.as_ptr() as usize,
            PAGE_SIZE * number_of_pages,
            crate::memory::page_tables::XWRMode::ReadWrite,
            "Heap".to_string(),
        );
        self.allocated_pages.push(pages);
        let ptr = core::ptr::without_provenance_mut(self.free_mmap_address);
        self.free_mmap_address += number_of_pages * PAGE_SIZE;
        ptr
    }

    pub fn add_notify_on_die(&mut self, pid: Pid) {
        self.notify_on_die.insert(pid);
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

    pub fn set_in_kernel_mode(&mut self, in_kernel_mode: bool) {
        self.in_kernel_mode = in_kernel_mode;
    }

    pub fn get_in_kernel_mode(&self) -> bool {
        self.in_kernel_mode
    }

    pub fn set_waiting_on_syscall<RetType: 'static>(&mut self) {
        self.state = ProcessState::Waiting;
        self.waiting_on_syscall = Some(core::any::TypeId::of::<RetType>());
    }

    pub fn resume_on_syscall<RetType: 'static>(&mut self, return_value: RetType) {
        assert_eq!(
            self.waiting_on_syscall,
            Some(core::any::TypeId::of::<RetType>()),
            "resume return type is different than expected"
        );
        let ptr = self.register_state[Register::a2] as *mut RetType;
        assert!(!ptr.is_null() && ptr.is_aligned());
        assert!(self.page_table.is_valid_userspace_ptr(ptr, true));
        let kernel_ptr = self
            .page_table
            .translate_userspace_address_to_physical_address(ptr)
            .expect("Return pointer must be valid");

        // SAFETY: We assured safety in the above checks
        unsafe {
            kernel_ptr.write(return_value);
        }

        self.waiting_on_syscall = None;
        self.state = ProcessState::Runnable;
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
            notify_on_die: BTreeSet::new(),
            waiting_on_syscall: None,
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
