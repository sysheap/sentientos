use alloc::{string::String, sync::Arc};
use core::{any::TypeId, ffi::c_int};

use common::{
    mutex::Mutex,
    pid::{Pid, Tid},
    syscalls::trap_frame::{Register, TrapFrame},
};

use crate::processes::userspace_ptr::UserspacePtr;

use super::process::{ProcessRef, ProcessWeakRef};

pub type ThreadRef = Arc<Mutex<Thread>>;

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
    waiting_on_syscall: Option<TypeId>,
    process: ProcessWeakRef,
    clear_child_tid: Option<UserspacePtr<*mut c_int>>,
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
        }))
    }

    pub fn get_tid(&self) -> Tid {
        self.tid
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

    pub fn set_waiting_on_syscall<RetType: 'static>(&mut self) {
        self.state = ThreadState::Waiting;
        self.waiting_on_syscall = Some(core::any::TypeId::of::<RetType>());
    }

    pub fn process(&self) -> ProcessRef {
        self.process
            .upgrade()
            .expect("Process must always be alive when thread exists")
    }

    pub fn resume_on_syscall<RetType: 'static>(&mut self, return_value: RetType) {
        assert_eq!(
            self.waiting_on_syscall,
            Some(core::any::TypeId::of::<RetType>()),
            "resume return type is different than expected"
        );
        let ptr = self.register_state[Register::a2] as *mut RetType;
        assert!(!ptr.is_null() && ptr.is_aligned());

        self.process().with_lock(|p| {
            assert!(p.get_page_table().is_valid_userspace_ptr(ptr, true));
            let kernel_ptr = p
                .get_page_table()
                .translate_userspace_address_to_physical_address(ptr)
                .expect("Return pointer must be valid");

            // SAFETY: We assured safety in the above checks
            unsafe {
                kernel_ptr.write(return_value);
            }
        });

        self.waiting_on_syscall = None;
        self.state = ThreadState::Runnable;
    }
}
