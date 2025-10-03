use super::process::{ProcessRef, ProcessWeakRef};
use crate::processes::userspace_ptr::{ContainsUserspacePtr, UserspacePtr};
use alloc::{
    boxed::Box,
    string::String,
    sync::{Arc, Weak},
};
use common::{
    mutex::Mutex,
    pid::{Pid, Tid},
    syscalls::trap_frame::{Register, TrapFrame},
};
use core::{
    ffi::{c_int, c_uint},
    fmt::Debug,
    ptr::null_mut,
};
use headers::{
    errno::Errno,
    syscall_types::{_NSIG, sigaction, sigaltstack, sigset_t, stack_t},
};

pub type ThreadRef = Arc<Mutex<Thread>>;
pub type ThreadWeakRef = Weak<Mutex<Thread>>;

pub struct SyscallFinalizer(Box<dyn Fn() -> Result<isize, Errno> + Send + 'static>);

impl SyscallFinalizer {
    fn call(self) -> Result<isize, Errno> {
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
    Waiting,
}

#[derive(Debug)]
pub struct Thread {
    tid: Tid,
    pid: Pid,
    process_name: Arc<String>,
    register_state: TrapFrame,
    program_counter: usize,
    state: ThreadState,
    in_kernel_mode: bool,
    waiting_on_syscall: Option<SyscallFinalizer>,
    process: ProcessWeakRef,
    clear_child_tid: Option<UserspacePtr<*mut c_int>>,
    sigaltstack: ContainsUserspacePtr<stack_t>,
    sigmask: sigset_t,
    sigaction: [sigaction; _NSIG as usize],
}

impl core::fmt::Display for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "tid={} pid={} process_name={} pc={:#x} state={:?} in_kernel_mode={} waiting_on_syscall={}",
            self.tid,
            self.pid,
            self.process_name,
            self.program_counter,
            self.state,
            self.in_kernel_mode,
            self.waiting_on_syscall.is_some()
        )
    }
}

impl Thread {
    pub fn new(
        tid: Tid,
        pid: Pid,
        process_name: Arc<String>,
        register_state: TrapFrame,
        program_counter: usize,
        in_kernel_mode: bool,
        process: ProcessWeakRef,
    ) -> ThreadRef {
        Arc::new(Mutex::new(Self {
            tid,
            pid,
            process_name,
            register_state,
            program_counter,
            state: ThreadState::Runnable,
            in_kernel_mode,
            waiting_on_syscall: None,
            process,
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

    pub fn get_tid(&self) -> Tid {
        self.tid
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

    pub fn set_in_kernel_mode(&mut self, in_kernel_mode: bool) {
        self.in_kernel_mode = in_kernel_mode;
    }

    pub fn get_in_kernel_mode(&self) -> bool {
        self.in_kernel_mode
    }

    pub fn set_waiting_on_syscall_linux(
        &mut self,
        finalizer: impl Fn() -> Result<isize, Errno> + Send + 'static,
    ) {
        self.state = ThreadState::Waiting;
        self.waiting_on_syscall = Some(SyscallFinalizer(Box::new(finalizer)));
    }

    pub fn has_process(&self) -> bool {
        self.process.strong_count() > 0
    }

    pub fn process(&self) -> ProcessRef {
        // self.process
        //     .upgrade()
        //     .expect("Process must always be alive when thread exists")
        match self.process.upgrade() {
            Some(p) => p,
            None => {
                panic!(
                    "Process {} non existent tid={} pid={}",
                    self.process_name, self.tid, self.pid
                );
            }
        }
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
