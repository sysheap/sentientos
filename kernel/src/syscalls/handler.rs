use common::pid::Tid;

use crate::{
    cpu::Cpu,
    debug,
    processes::{process::ProcessRef, thread::ThreadRef},
};

pub(super) struct SyscallHandler {
    current_process: ProcessRef,
    current_thread: ThreadRef,
    current_tid: Tid,
}

impl SyscallHandler {
    pub(super) fn new() -> Self {
        let current_thread = Cpu::with_scheduler(|s| s.get_current_thread().clone());
        let (current_tid, current_process) =
            current_thread.with_lock(|t| (t.get_tid(), t.process()));
        Self {
            current_process,
            current_thread,
            current_tid,
        }
    }

    pub fn current_tid(&self) -> Tid {
        self.current_tid
    }

    pub fn current_process(&self) -> &ProcessRef {
        &self.current_process
    }

    pub fn current_thread(&self) -> &ThreadRef {
        &self.current_thread
    }

    pub fn sys_exit(&mut self, status: isize) {
        let exit_status = i32::try_from(status).expect("exit status fits in i32");

        Cpu::with_scheduler(|mut s| {
            s.kill_current_process(exit_status);
        });

        debug!("Exit process with status: {status}\n");
    }
}
