use crate::{
    debug,
    klibc::elf::ElfFile,
    memory::{page::PinnedHeapPages, page_tables::RootPageTableHolder, PAGE_SIZE},
    net::sockets::SharedAssignedSocket,
    processes::loader::{self, LoadedElf, STACK_END, STACK_START},
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use common::{
    errors::LoaderError,
    mutex::Mutex,
    net::UDPDescriptor,
    pid::{Pid, Tid},
    syscalls::trap_frame::{Register, TrapFrame},
    util::align_down,
};
use core::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

use super::thread::{Thread, ThreadRef};

pub const POWERSAVE_PID: Pid = Pid(0);
pub const POWERSAVE_TID: Tid = Tid(0);

const FREE_MMAP_START_ADDRESS: usize = 0x2000000000;

pub type ProcessRef = Arc<Mutex<Process>>;
pub type ProcessWeakRef = Weak<Mutex<Process>>;

fn get_next_id() -> u64 {
    // PIDs will start from 1
    // 0 is reserved for the powersave process
    static PID_COUNTER: AtomicU64 = AtomicU64::new(1);
    let next_pid = PID_COUNTER.fetch_add(1, Ordering::Relaxed);
    assert_ne!(next_pid, u64::MAX, "We ran out of process pids");
    next_pid
}

pub struct Process {
    name: Arc<String>,
    pid: Pid,
    page_table: RootPageTableHolder,
    allocated_pages: Vec<PinnedHeapPages>,
    free_mmap_address: usize,
    next_free_descriptor: u64,
    open_udp_sockets: BTreeMap<UDPDescriptor, SharedAssignedSocket>,
    notify_on_die: BTreeSet<Tid>,
    threads: BTreeMap<Tid, ThreadRef>,
}

impl Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Process [
            PID: {},
            Page Table: {:?},
            Number of allocated pages: {},
            Threads: {:?}
        ]",
            self.pid,
            self.page_table,
            self.allocated_pages.len(),
            self.threads,
        )
    }
}

impl Process {
    fn new(
        name: impl Into<String>,
        pid: Pid,
        register_state: TrapFrame,
        page_table: RootPageTableHolder,
        program_counter: usize,
        allocated_pages: Vec<PinnedHeapPages>,
        in_kernel_mode: bool,
    ) -> Arc<Mutex<Self>> {
        let name = Arc::new(name.into());
        let process = Arc::new(Mutex::new(Self {
            name: name.clone(),
            pid,
            page_table,
            allocated_pages,
            free_mmap_address: FREE_MMAP_START_ADDRESS,
            next_free_descriptor: 0,
            open_udp_sockets: BTreeMap::new(),
            notify_on_die: BTreeSet::new(),
            threads: BTreeMap::new(),
        }));

        let main_thread_tid = Tid(pid.0);
        let main_thread = Thread::new(
            main_thread_tid,
            pid,
            name,
            register_state,
            program_counter,
            in_kernel_mode,
            Arc::downgrade(&process),
        );

        process.lock().threads.insert(main_thread_tid, main_thread);

        process
    }

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

        page_table.map(
            STACK_END,
            stack_addr.get(),
            PAGE_SIZE,
            crate::memory::page_tables::XWRMode::ReadWrite,
            false,
            "Stack".to_string(),
        );

        let mut register_state = TrapFrame::zero();
        register_state[Register::sp] = STACK_START;

        Self::new(
            "powersave",
            POWERSAVE_PID,
            register_state,
            page_table,
            powersave as usize,
            allocated_pages,
            true,
        )
    }

    pub fn threads(&self) -> impl Iterator<Item = &ThreadRef> {
        self.threads.values()
    }

    pub fn threads_len(&self) -> usize {
        self.threads.len()
    }

    pub fn get_notifies_on_die(&self) -> impl Iterator<Item = &Tid> {
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

    pub fn add_notify_on_die(&mut self, tid: Tid) {
        self.notify_on_die.insert(tid);
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

    pub fn main_thread(&self) -> ThreadRef {
        self.threads
            .get(&Tid(self.pid.0))
            .cloned()
            .expect("Main thread must always exist")
    }

    pub fn from_elf(
        elf_file: &ElfFile,
        name: &str,
        args: &[&str],
    ) -> Result<Arc<Mutex<Self>>, LoaderError> {
        debug!("Create process from elf file");

        let LoadedElf {
            entry_address,
            page_tables: page_table,
            allocated_pages,
            args_start,
        } = loader::load_elf(elf_file, name, args)?;

        let mut register_state = TrapFrame::zero();
        register_state[Register::a0] = args_start;
        register_state[Register::sp] = align_down(args_start - 1, 8);

        Ok(Self::new(
            name,
            Pid(get_next_id()),
            register_state,
            page_table,
            entry_address,
            allocated_pages,
            false,
        ))
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

impl core::fmt::Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "pid={} name={}", self.pid, self.name)?;
        for thread in self.threads.values() {
            writeln!(f, "\t{}", *thread.lock())?;
        }
        Ok(())
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
    use common::{pid::Tid, syscalls::trap_frame::Register};

    use crate::{
        autogenerated::userspace_programs::PROG1, klibc::elf::ElfFile, memory::PAGE_SIZE,
        processes::process::FREE_MMAP_START_ADDRESS,
    };
    use alloc::sync::Arc;

    use super::Process;

    #[test_case]
    fn create_process_from_elf() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let _process = Process::from_elf(&elf, "prog1", &[]);
    }

    #[cfg(not(miri))]
    #[test_case]
    fn create_process_from_elf_with_args() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let process_ref = Process::from_elf(&elf, "prog1", &["arg1", "arg2"]).unwrap();
        let mut process = Arc::into_inner(process_ref).unwrap().into_inner();
        let pid = process.pid;
        let main_thread = Arc::into_inner(process.threads.remove(&Tid(pid.0)).unwrap())
            .unwrap()
            .into_inner();

        // a0 points to the start of the arguments
        let mut arg_ptr =
            core::ptr::without_provenance(main_thread.get_register_state()[Register::a0]);

        // Translate userspace ptr to kernel pointer
        arg_ptr = process
            .page_table
            .translate_userspace_address_to_physical_address(arg_ptr)
            .unwrap();

        // SAFTETY: Unsafe is okay in unit tests because we are checking the
        // behavior anyways.
        unsafe {
            let name = core::ffi::CStr::from_ptr(arg_ptr).to_str().unwrap();
            assert_eq!(name, "prog1");
            arg_ptr = arg_ptr.add(name.len() + 1);

            let arg1 = core::ffi::CStr::from_ptr(arg_ptr).to_str().unwrap();
            assert_eq!(arg1, "arg1");
            arg_ptr = arg_ptr.add(arg1.len() + 1);

            let arg2 = core::ffi::CStr::from_ptr(arg_ptr).to_str().unwrap();
            assert_eq!(arg2, "arg2");
            arg_ptr = arg_ptr.add(arg2.len() + 1);

            let empty = core::ffi::CStr::from_ptr(arg_ptr).to_str().unwrap();
            assert_eq!(empty, "");
        }
    }

    #[test_case]
    fn mmap_process() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");

        let process_ref = Process::from_elf(&elf, "prog1", &[]).unwrap();

        let mut process = Arc::into_inner(process_ref).unwrap().into_inner();

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
