use super::process::ProcessRef;
use crate::{
    debug,
    klibc::elf::ElfFile,
    memory::{page::PinnedHeapPages, page_tables::RootPageTableHolder},
    processes::{
        brk::Brk,
        loader::{self, LoadedElf},
        process::{POWERSAVE_TID, Process},
        task::Task,
        userspace_ptr::{ContainsUserspacePtr, UserspacePtr},
    },
};
use alloc::{
    collections::BTreeSet,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use common::{
    errors::LoaderError,
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

use crate::klibc::Spinlock;

pub type ThreadRef = Arc<Spinlock<Thread>>;
pub type ThreadWeakRef = Weak<Spinlock<Thread>>;

pub type SyscallTask = Task<Result<isize, Errno>>;

fn get_next_tid() -> Tid {
    // PIDs will start from 1
    // 0 is reserved for the powersave process
    static TID_COUNTER: AtomicU64 = AtomicU64::new(1);
    let next_tid = TID_COUNTER.fetch_add(1, Ordering::Relaxed);
    assert_ne!(next_tid, u64::MAX, "We ran out of process pids");
    Tid(next_tid)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Running { cpu_id: usize },
    Runnable,
    Waiting,
}

#[derive(Debug)]
struct SignalState {
    sigaltstack: ContainsUserspacePtr<stack_t>,
    sigmask: sigset_t,
    sigaction: [sigaction; _NSIG as usize],
}

impl SignalState {
    fn new() -> Self {
        Self {
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
        }
    }
}

#[derive(Debug)]
pub struct Thread {
    tid: Tid,
    process_name: Arc<String>,
    register_state: TrapFrame,
    program_counter: usize,
    state: ThreadState,
    wakeup_pending: bool,
    in_kernel_mode: bool,
    process: ProcessRef,
    notify_on_die: BTreeSet<Tid>,
    clear_child_tid: Option<UserspacePtr<*mut c_int>>,
    signal_state: SignalState,
    syscall_task: Option<SyscallTask>,
}

impl core::fmt::Display for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "tid={} process_name={} pc={:#x} state={:?} wakeup_pending={} in_kernel_mode={}",
            self.tid,
            self.process_name,
            self.program_counter,
            self.state,
            self.wakeup_pending,
            self.in_kernel_mode,
        )
    }
}

impl Thread {
    pub fn create_powersave_thread() -> Arc<Spinlock<Self>> {
        // SAFETY: powersave is defined in powersave.S; it runs wfi in a loop.
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
            powersave as *const () as usize,
            allocated_pages,
            true,
            Brk::empty(),
            POWERSAVE_TID,
        )
    }

    pub fn from_elf(
        elf_file: &ElfFile,
        name: &str,
        args: &[&str],
        parent_tid: Tid,
    ) -> Result<Arc<Spinlock<Self>>, LoaderError> {
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
            parent_tid,
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
        parent_tid: Tid,
    ) -> ThreadRef {
        let name = Arc::new(name.into());
        let process = Arc::new(Spinlock::new(Process::new(
            name.clone(),
            page_table,
            allocated_pages,
            brk,
            tid,
            parent_tid,
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
        Arc::new(Spinlock::new(Self {
            tid,
            process_name,
            register_state,
            program_counter,
            state: ThreadState::Runnable,
            wakeup_pending: false,
            in_kernel_mode,
            process,
            notify_on_die: BTreeSet::new(),
            clear_child_tid: None,
            signal_state: SignalState::new(),
            syscall_task: None,
        }))
    }

    pub fn get_name(&self) -> &str {
        &self.process_name
    }

    pub fn set_syscall_task_and_suspend(&mut self, task: SyscallTask) {
        assert!(self.syscall_task.is_none(), "syscall task is already set");
        self.syscall_task = Some(task);
        if self.wakeup_pending {
            // A waker fired between poll() returning Pending and now.
            // The thread is still Running so wake_up() couldn't transition
            // it to Runnable. Don't suspend — stay schedulable.
            self.wakeup_pending = false;
        } else {
            self.suspend();
        }
    }

    pub fn wake_up(&mut self) {
        if self.state == ThreadState::Waiting {
            self.state = ThreadState::Runnable;
        } else if matches!(self.state, ThreadState::Running { .. }) {
            // Waker fired while thread is Running (between poll() and suspend()).
            // Record it so set_syscall_task_and_suspend() knows not to sleep.
            self.wakeup_pending = true;
        }
        // If Runnable, the thread will be scheduled and re-poll naturally.
    }

    fn suspend(&mut self) {
        assert_ne!(
            self.state,
            ThreadState::Waiting,
            "Thread should not be already in waiting state"
        );
        self.state = ThreadState::Waiting;
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
            &mut self.signal_state.sigaction[sig as usize],
            sigaction,
        ))
    }

    pub fn get_sigaction(&self, sig: c_uint) -> Result<sigaction, Errno> {
        if sig >= _NSIG {
            return Err(Errno::EINVAL);
        }
        Ok(self.signal_state.sigaction[sig as usize])
    }

    pub fn get_sigset(&self) -> sigset_t {
        self.signal_state.sigmask
    }

    pub fn set_sigset(&mut self, sigmask: sigset_t) -> sigset_t {
        core::mem::replace(&mut self.signal_state.sigmask, sigmask)
    }

    pub fn get_sigaltstack(&self) -> sigaltstack {
        self.signal_state.sigaltstack.0
    }

    pub fn set_sigaltstack(&mut self, sigaltstack: &sigaltstack) {
        self.signal_state.sigaltstack.0 = *sigaltstack;
    }

    pub fn clear_wakeup_pending(&mut self) {
        self.wakeup_pending = false;
    }

    pub fn take_syscall_task(&mut self) -> Option<SyscallTask> {
        self.syscall_task.take()
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

    pub fn get_register_state_mut(&mut self) -> &mut TrapFrame {
        &mut self.register_state
    }

    pub fn set_register_state(&mut self, register_state: TrapFrame) {
        self.register_state = register_state;
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

    pub fn process(&self) -> ProcessRef {
        self.process.clone()
    }
}
