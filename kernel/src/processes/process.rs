use crate::{
    debug,
    klibc::elf::ElfFile,
    memory::{PAGE_SIZE, page::PinnedHeapPages, page_tables::RootPageTableHolder},
    net::sockets::SharedAssignedSocket,
    processes::{
        brk::Brk,
        loader::{self, LoadedElf, STACK_END, STACK_SIZE, STACK_SIZE_PAGES, STACK_START},
        userspace_ptr::UserspacePtr,
    },
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
    pointer::Pointer,
    syscalls::trap_frame::{Register, TrapFrame},
};
use core::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};
use headers::errno::Errno;

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
    brk: Brk,
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
    #[allow(clippy::too_many_arguments)]
    fn new(
        name: impl Into<String>,
        pid: Pid,
        register_state: TrapFrame,
        page_table: RootPageTableHolder,
        program_counter: usize,
        allocated_pages: Vec<PinnedHeapPages>,
        in_kernel_mode: bool,
        brk: Brk,
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
            brk,
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

    pub fn brk(&mut self, brk: usize) -> usize {
        self.brk.brk(brk)
    }

    pub fn read_userspace_slice<T: Clone>(
        &self,
        ptr: &UserspacePtr<*const T>,
        len: usize,
    ) -> Result<Vec<T>, Errno> {
        let kernel_ptr = self.get_kernel_space_fat_pointer(ptr, len)?;
        // SAFETY: We just validate the pointer
        let slice = unsafe { core::slice::from_raw_parts(kernel_ptr, len) };
        Ok(slice.to_vec())
    }

    pub fn read_userspace_str(
        &self,
        ptr: &UserspacePtr<*const u8>,
        len: usize,
    ) -> Result<String, Errno> {
        let kernel_ptr = self.get_kernel_space_fat_pointer(ptr, len)?;
        // SAFETY: We just validate the pointer
        let slice = unsafe { core::slice::from_raw_parts(kernel_ptr, len) };
        let cow = String::from_utf8_lossy(slice);
        Ok(cow.into_owned())
    }

    fn get_kernel_space_pointer<PTR: Pointer>(
        &self,
        ptr: &UserspacePtr<PTR>,
    ) -> Result<PTR, Errno> {
        let pt = self.get_page_table();
        // SAFETY: We know it is a userspace pointer and we gonna translate it later
        let ptr = unsafe { ptr.get() };
        if !pt.is_valid_userspace_ptr(ptr, PTR::WRITABLE) {
            return Err(Errno::EFAULT);
        }
        pt.translate_userspace_address_to_physical_address(ptr)
            .ok_or(Errno::EFAULT)
    }

    fn get_kernel_space_fat_pointer<PTR: Pointer>(
        &self,
        ptr: &UserspacePtr<PTR>,
        len: usize,
    ) -> Result<PTR, Errno> {
        let pt = self.get_page_table();
        // SAFETY: We know it is a userspace pointer and we gonna translate it later
        let ptr = unsafe { ptr.get() };
        if !pt.is_valid_userspace_fat_ptr(ptr, len, PTR::WRITABLE) {
            return Err(Errno::EFAULT);
        }
        pt.translate_userspace_address_to_physical_address(ptr)
            .ok_or(Errno::EFAULT)
    }

    pub fn read_userspace_ptr<T>(&self, ptr: &UserspacePtr<*const T>) -> Result<T, Errno> {
        let kernel_ptr = self.get_kernel_space_pointer(ptr)?;
        // SAFETY: We just validate the pointer
        unsafe { Ok(kernel_ptr.read()) }
    }

    pub fn write_userspace_ptr<T>(
        &self,
        ptr: &UserspacePtr<*mut T>,
        value: T,
    ) -> Result<(), Errno> {
        let kernel_ptr = self.get_kernel_space_pointer(ptr)?;
        // SAFETY: We just validate the pointer
        unsafe {
            kernel_ptr.write(value);
        }
        Ok(())
    }

    pub fn create_powersave_process() -> Arc<Mutex<Self>> {
        unsafe extern "C" {
            fn powersave();
        }

        let mut allocated_pages = Vec::with_capacity(1);

        // Map 4KB stack
        let stack = PinnedHeapPages::new(STACK_SIZE_PAGES);
        let stack_addr = stack.addr();
        allocated_pages.push(stack);

        let mut page_table = RootPageTableHolder::new_with_kernel_mapping(false);

        page_table.map(
            STACK_END,
            stack_addr,
            STACK_SIZE,
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
            Brk::empty(),
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
            brk,
        } = loader::load_elf(elf_file, name, args)?;

        let mut register_state = TrapFrame::zero();
        register_state[Register::a0] = args_start;
        register_state[Register::sp] = args_start;

        Ok(Self::new(
            name,
            Pid(get_next_id()),
            register_state,
            page_table,
            entry_address,
            allocated_pages,
            false,
            brk,
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
    use common::{consumable_buffer::ConsumableBuffer, pid::Tid, syscalls::trap_frame::Register};

    use crate::{
        autogenerated::userspace_programs::PROG1,
        klibc::elf::ElfFile,
        memory::PAGE_SIZE,
        processes::{
            loader::{STACK_END, STACK_START},
            process::FREE_MMAP_START_ADDRESS,
        },
    };
    use alloc::sync::Arc;

    use super::Process;

    #[test_case]
    fn create_process_from_elf() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let _process = Process::from_elf(&elf, "prog1", &[]);
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
