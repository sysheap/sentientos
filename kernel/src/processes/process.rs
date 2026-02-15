use crate::{
    debug,
    memory::{
        PAGE_SIZE,
        page::PinnedHeapPages,
        page_tables::{RootPageTableHolder, XWRMode},
    },
    processes::{brk::Brk, fd_table::FdTable, thread::ThreadWeakRef, userspace_ptr::UserspacePtr},
};
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use common::{pid::Tid, pointer::Pointer};
use core::{self, fmt::Debug, ptr::null_mut};
use headers::errno::Errno;

use crate::klibc::Spinlock;

use super::thread::ThreadRef;

pub const POWERSAVE_TID: Tid = Tid(0);

const FREE_MMAP_START_ADDRESS: usize = 0x2000000000;

pub type ProcessRef = Arc<Spinlock<Process>>;

pub struct Process {
    name: Arc<String>,
    page_table: RootPageTableHolder,
    allocated_pages: Vec<PinnedHeapPages>,
    mmap_allocations: BTreeMap<usize, PinnedHeapPages>,
    free_mmap_address: usize,
    fd_table: FdTable,
    threads: BTreeMap<Tid, ThreadWeakRef>,
    main_tid: Tid,
    parent_tid: Tid,
    exit_status: i32,
    brk: Brk,
}

impl Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Process [
            Page Table: {:?},
            Number of allocated pages: {},
            Threads: {:?}
        ]",
            self.page_table,
            self.allocated_pages.len(),
            self.threads,
        )
    }
}

impl Process {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Arc<String>,
        page_table: RootPageTableHolder,
        allocated_pages: Vec<PinnedHeapPages>,
        brk: Brk,
        main_thread: Tid,
        parent_tid: Tid,
    ) -> Self {
        Self {
            name,
            page_table,
            allocated_pages,
            mmap_allocations: BTreeMap::new(),
            free_mmap_address: FREE_MMAP_START_ADDRESS,
            fd_table: FdTable::new(),
            threads: BTreeMap::new(),
            brk,
            main_tid: main_thread,
            parent_tid,
            exit_status: 0,
        }
    }

    pub fn brk(&mut self, brk: usize) -> usize {
        self.brk.brk(brk)
    }

    pub fn add_thread(&mut self, tid: Tid, thread: ThreadWeakRef) {
        assert!(
            self.threads.insert(tid, thread).is_none(),
            "Duplicate TID {tid} in process"
        );
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

    pub fn write_userspace_slice<T: Copy>(
        &self,
        ptr: &UserspacePtr<*mut T>,
        data: &[T],
    ) -> Result<(), Errno> {
        let len = data.len();
        let kernel_ptr = self.get_kernel_space_fat_pointer(ptr, len)?;
        // SAFETY: We just validate the pointer
        let slice = unsafe { core::slice::from_raw_parts_mut(kernel_ptr, len) };
        slice.copy_from_slice(data);
        Ok(())
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

    pub fn threads_len(&self) -> usize {
        self.threads.len()
    }

    pub fn mmap_pages_with_address(
        &mut self,
        number_of_pages: usize,
        addr: usize,
        permission: XWRMode,
    ) -> *mut u8 {
        let length = number_of_pages * PAGE_SIZE;
        if self.page_table.is_mapped(addr..addr + length) {
            return null_mut();
        }
        let pages = PinnedHeapPages::new(number_of_pages);
        self.page_table
            .map_userspace(addr, pages.addr(), length, permission, "mmap".into());
        self.mmap_allocations.insert(addr, pages);
        core::ptr::without_provenance_mut(addr)
    }

    pub fn mmap_pages(&mut self, number_of_pages: usize, permission: XWRMode) -> *mut u8 {
        let pages = PinnedHeapPages::new(number_of_pages);
        let addr = self.free_mmap_address;
        self.page_table.map_userspace(
            addr,
            pages.as_ptr() as usize,
            PAGE_SIZE * number_of_pages,
            permission,
            "mmap".to_string(),
        );
        self.mmap_allocations.insert(addr, pages);
        self.free_mmap_address += number_of_pages * PAGE_SIZE;
        core::ptr::without_provenance_mut(addr)
    }

    pub fn munmap_pages(&mut self, addr: usize, length: usize) -> Result<(), Errno> {
        let pages = self.mmap_allocations.remove(&addr).ok_or(Errno::EINVAL)?;
        if pages.size() != length {
            self.mmap_allocations.insert(addr, pages);
            return Err(Errno::EINVAL);
        }
        self.page_table.unmap_userspace(addr, length);
        Ok(())
    }

    pub fn get_page_table(&self) -> &RootPageTableHolder {
        &self.page_table
    }

    pub fn get_page_table_mut(&mut self) -> &mut RootPageTableHolder {
        &mut self.page_table
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn main_thread(&self) -> ThreadRef {
        let main_thread = self
            .threads
            .get(&self.main_tid)
            .cloned()
            .expect("Main thread must always exist");
        ThreadWeakRef::upgrade(&main_thread).expect("Main thread must always exist")
    }

    pub fn main_tid(&self) -> Tid {
        self.main_tid
    }

    pub fn parent_tid(&self) -> Tid {
        self.parent_tid
    }

    pub fn set_parent_tid(&mut self, parent_tid: Tid) {
        self.parent_tid = parent_tid;
    }

    pub fn exit_status(&self) -> i32 {
        self.exit_status
    }

    pub fn set_exit_status(&mut self, status: i32) {
        self.exit_status = status;
    }

    pub fn fd_table(&self) -> &FdTable {
        &self.fd_table
    }

    pub fn fd_table_mut(&mut self) -> &mut FdTable {
        &mut self.fd_table
    }
}

impl core::fmt::Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "main_tid={} name={}", self.main_tid, self.name)?;
        for thread in self.threads.values().filter_map(ThreadWeakRef::upgrade) {
            writeln!(f, "\t{}", *thread.lock())?;
        }
        Ok(())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        debug!(
            "Drop process (MAIN_TID: {}) (Allocated pages: {:?})",
            self.main_tid, self.allocated_pages
        );
    }
}

#[cfg(test)]
mod tests {
    use common::{pid::Tid, syscalls::trap_frame::Register};

    use crate::{
        autogenerated::userspace_programs::PROG1,
        klibc::{consumable_buffer::ConsumableBuffer, elf::ElfFile},
        memory::{PAGE_SIZE, page_tables::XWRMode},
        processes::{
            loader::{STACK_END, STACK_START},
            process::FREE_MMAP_START_ADDRESS,
            thread::Thread,
        },
    };
    use alloc::sync::Arc;

    use super::Process;

    #[test_case]
    fn create_process_from_elf() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let _process =
            Thread::from_elf(&elf, "prog1", &[], Tid(0)).expect("ELF loading must succeed");
    }

    #[test_case]
    fn mmap_process() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");

        let process_ref =
            Thread::from_elf(&elf, "prog1", &[], Tid(0)).expect("ELF loading must succeed");

        let thread = Arc::into_inner(process_ref)
            .expect("Must be sole owner")
            .into_inner();
        let process = thread.process();
        let mut process = process.lock();

        assert!(
            process.free_mmap_address == FREE_MMAP_START_ADDRESS,
            "Free MMAP Address must set to correct start"
        );
        let ptr = process.mmap_pages(1, XWRMode::ReadWrite);
        assert!(
            ptr as usize == FREE_MMAP_START_ADDRESS,
            "Returned pointer must have the value of the initial free mmap start address."
        );
        assert!(
            process.free_mmap_address == FREE_MMAP_START_ADDRESS + PAGE_SIZE,
            "Free mmap address must have the value of the next free value"
        );
        let ptr = process.mmap_pages(2, XWRMode::ReadWrite);
        assert!(
            ptr as usize == FREE_MMAP_START_ADDRESS + PAGE_SIZE,
            "Returned pointer must have the value of the initial free mmap start address."
        );
        assert!(
            process.free_mmap_address == FREE_MMAP_START_ADDRESS + (3 * PAGE_SIZE),
            "Free mmap address must have the value of the next free value"
        );
    }

    #[test_case]
    fn munmap_process() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let process_ref =
            Thread::from_elf(&elf, "prog1", &[], Tid(0)).expect("ELF loading must succeed");
        let thread = Arc::into_inner(process_ref)
            .expect("Must be sole owner")
            .into_inner();
        let process = thread.process();
        let mut process = process.lock();

        let ptr = process.mmap_pages(1, XWRMode::ReadWrite);
        let addr = ptr as usize;
        assert!(process.get_page_table().is_mapped(addr..addr + PAGE_SIZE));

        process
            .munmap_pages(addr, PAGE_SIZE)
            .expect("munmap must succeed");
        assert!(!process.get_page_table().is_mapped(addr..addr + PAGE_SIZE));
    }

    #[test_case]
    fn munmap_unknown_address_returns_einval() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let process_ref =
            Thread::from_elf(&elf, "prog1", &[], Tid(0)).expect("ELF loading must succeed");
        let thread = Arc::into_inner(process_ref)
            .expect("Must be sole owner")
            .into_inner();
        let process = thread.process();
        let mut process = process.lock();

        let result = process.munmap_pages(0xDEAD_0000, PAGE_SIZE);
        assert_eq!(result, Err(headers::errno::Errno::EINVAL));
    }

    #[test_case]
    fn munmap_wrong_length_returns_einval() {
        let elf = ElfFile::parse(PROG1).expect("Cannot parse elf file");
        let process_ref =
            Thread::from_elf(&elf, "prog1", &[], Tid(0)).expect("ELF loading must succeed");
        let thread = Arc::into_inner(process_ref)
            .expect("Must be sole owner")
            .into_inner();
        let process = thread.process();
        let mut process = process.lock();

        let ptr = process.mmap_pages(1, XWRMode::ReadWrite);
        let addr = ptr as usize;
        let result = process.munmap_pages(addr, PAGE_SIZE * 2);
        assert_eq!(result, Err(headers::errno::Errno::EINVAL));
        assert!(
            process.get_page_table().is_mapped(addr..addr + PAGE_SIZE),
            "mapping must still exist after failed munmap"
        );
    }
}
