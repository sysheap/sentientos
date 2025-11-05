use super::process::ProcessRef;
use crate::{
    debug,
    klibc::elf::ElfFile,
    memory::{page::PinnedHeapPages, page_tables::RootPageTableHolder},
    processes::{
        brk::Brk,
        loader::{self, LoadedElf},
        process::{POWERSAVE_TID, Process},
        userspace_ptr::{ContainsUserspacePtr, UserspacePtr},
    },
};
use alloc::{
    boxed::Box,
    collections::BTreeSet,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use common::{
    errors::LoaderError,
    mutex::Mutex,
    pid::Tid,
    syscalls::trap_frame::{Register, TrapFrame},
};
use core::{
    ffi::{c_int, c_uint},
    fmt::Debug,
    ptr::null_mut,
    sync::atomic::{AtomicU64, Ordering},
};
use headers::{
    errno::Errno,
    syscall_types::{_NSIG, sigaction, sigaltstack, sigset_t, stack_t},
};

pub type ThreadRef = Arc<Mutex<Thread>>;
pub type ThreadWeakRef = Weak<Mutex<Thread>>;

fn get_next_tid() -> Tid {
    // PIDs will start from 1
    // 0 is reserved for the powersave process
    static TID_COUNTER: AtomicU64 = AtomicU64::new(1);
    let next_tid = TID_COUNTER.fetch_add(1, Ordering::Relaxed);
    assert_ne!(next_tid, u64::MAX, "We ran out of process pids");
    Tid(next_tid)
}

pub struct SyscallFinalizer(Box<dyn FnMut() -> Result<isize, Errno> + Send + 'static>);

impl SyscallFinalizer {
    fn call(mut self) -> Result<isize, Errno> {
        self.0()
    }
}

impl Debug for SyscallFinalizer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("SyscallFinalizer").finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Running,
    Runnable,
    Waiting, // Waiting or killed
}

#[derive(Debug)]
pub struct Thread {
    tid: Tid,
    process_name: Arc<String>,
    register_state: TrapFrame,
    program_counter: usize,
    state: ThreadState,
    in_kernel_mode: bool,
    waiting_on_syscall: Option<SyscallFinalizer>,
    process: ProcessRef,
    notify_on_die: BTreeSet<Tid>,
    clear_child_tid: Option<UserspacePtr<*mut c_int>>,
    sigaltstack: ContainsUserspacePtr<stack_t>,
    sigmask: sigset_t,
    sigaction: [sigaction; _NSIG as usize],
}

impl core::fmt::Display for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "tid={} process_name={} pc={:#x} state={:?} in_kernel_mode={} waiting_on_syscall={}",
            self.tid,
            self.process_name,
            self.program_counter,
            self.state,
            self.in_kernel_mode,
            self.waiting_on_syscall.is_some()
        )
    }
}

impl Thread {
    pub fn create_powersave_thread() -> Arc<Mutex<Self>> {
        unsafe extern "C" {
            fn powersave();
        }

        let allocated_pages = vec![];

        let page_table = RootPageTableHolder::new_with_kernel_mapping(false);

        let register_state = TrapFrame::zero();

        Self::new_process(
            "powersave",
            POWERSAVE_TID,
            register_state,
            page_table,
            powersave as usize,
            allocated_pages,
            true,
            Brk::empty(),
        )
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

        Ok(Self::new_process(
            name,
            get_next_tid(),
            register_state,
            page_table,
            entry_address,
            allocated_pages,
            false,
            brk,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_process(
        name: impl Into<String>,
        tid: Tid,
        register_state: TrapFrame,
        page_table: RootPageTableHolder,
        program_counter: usize,
        allocated_pages: Vec<PinnedHeapPages>,
        in_kernel_mode: bool,
        brk: Brk,
    ) -> ThreadRef {
        let name = Arc::new(name.into());
        let process = Arc::new(Mutex::new(Process::new(
            name.clone(),
            page_table,
            allocated_pages,
            brk,
            tid,
        )));

        let main_thread = Thread::new(
            tid,
            name,
            register_state,
            program_counter,
            in_kernel_mode,
            process.clone(),
        );

        process
            .lock()
            .add_thread(tid, ThreadRef::downgrade(&main_thread));

        main_thread
    }

    pub fn add_notify_on_die(&mut self, tid: Tid) {
        self.notify_on_die.insert(tid);
    }
    pub fn new(
        tid: Tid,
        process_name: Arc<String>,
        register_state: TrapFrame,
        program_counter: usize,
        in_kernel_mode: bool,
        process: ProcessRef,
    ) -> ThreadRef {
        Arc::new(Mutex::new(Self {
            tid,
            process_name,
            register_state,
            program_counter,
            state: ThreadState::Runnable,
            in_kernel_mode,
            waiting_on_syscall: None,
            process,
            notify_on_die: BTreeSet::new(),
            clear_child_tid: None,
            sigaltstack: ContainsUserspacePtr(sigaltstack {
                ss_sp: null_mut(),
                ss_flags: 0,
                ss_size: 0,
            }),
            sigmask: sigset_t { sig: [0] },
            sigaction: [sigaction {
                sa_handler: None,
                sa_flags: 0,
                sa_mask: sigset_t { sig: [0] },
            }; _NSIG as usize],
        }))
    }

    pub fn get_name(&self) -> &str {
        &self.process_name
    }

    pub fn get_tid(&self) -> Tid {
        self.tid
    }
    pub fn get_notifies_on_die(&self) -> impl Iterator<Item = &Tid> {
        self.notify_on_die.iter()
    }

    pub fn set_sigaction(&mut self, sig: c_uint, sigaction: sigaction) -> Result<sigaction, Errno> {
        if sig >= _NSIG {
            return Err(Errno::EINVAL);
        }
        Ok(core::mem::replace(
            &mut self.sigaction[sig as usize],
            sigaction,
        ))
    }

    pub fn get_sigaction(&self, sig: c_uint) -> Result<sigaction, Errno> {
        if sig >= _NSIG {
            return Err(Errno::EINVAL);
        }
        Ok(self.sigaction[sig as usize])
    }

    pub fn get_sigset(&self) -> sigset_t {
        self.sigmask
    }

    pub fn set_sigset(&mut self, sigmask: sigset_t) -> sigset_t {
        core::mem::replace(&mut self.sigmask, sigmask)
    }

    pub fn get_sigaltstack(&self) -> sigaltstack {
        self.sigaltstack.0
    }

    pub fn set_sigaltstack(&mut self, sigaltstack: &sigaltstack) {
        self.sigaltstack.0 = *sigaltstack;
    }

    pub fn set_clear_child_tid(&mut self, clear_child_tid: UserspacePtr<*mut c_int>) {
        self.clear_child_tid = Some(clear_child_tid);
    }

    pub fn get_clear_child_tid(&self) -> &Option<UserspacePtr<*mut c_int>> {
        &self.clear_child_tid
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

    pub fn get_state(&self) -> ThreadState {
        self.state
    }

    pub fn set_state(&mut self, state: ThreadState) {
        self.state = state;
    }

    pub fn get_in_kernel_mode(&self) -> bool {
        self.in_kernel_mode
    }

    pub fn set_waiting_on_syscall_linux(
        &mut self,
        finalizer: impl FnMut() -> Result<isize, Errno> + Send + 'static,
    ) {
        self.state = ThreadState::Waiting;
        self.waiting_on_syscall = Some(SyscallFinalizer(Box::new(finalizer)));
    }

    pub fn process(&self) -> ProcessRef {
        self.process.clone()
    }

    pub fn finalize_syscall(&mut self) {
        if let Some(finalizer) = self.waiting_on_syscall.take() {
            let ret = match finalizer.call() {
                Ok(ret) => ret,
                Err(errno) => -(errno as isize),
            };
            self.register_state[Register::a0] = ret as usize;
        }
    }

    pub fn resume_on_syscall_linux(&mut self) {
        self.state = ThreadState::Runnable;
    }
}
