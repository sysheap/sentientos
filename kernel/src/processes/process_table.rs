use alloc::{collections::BTreeMap, sync::Arc};
use common::mutex::Mutex;

use crate::{debug, info};

use super::process::{Pid, Process, ProcessState, NEVER_PID};

pub type ProcessRef = Arc<Mutex<Process>>;

pub struct ProcessTable {
    processes: BTreeMap<Pid, ProcessRef>,
}

impl ProcessTable {
    pub fn new() -> Self {
        let mut self_ = Self {
            processes: BTreeMap::new(),
        };
        self_.add_process(Process::never());
        self_
    }

    pub fn add_process(&mut self, process: Process) {
        self.processes
            .insert(process.get_pid(), Arc::new(Mutex::new(process)));
    }

    pub fn is_empty(&self) -> bool {
        // If only the never process is left
        // we know the process table is empty
        self.processes.len() == 1
    }

    pub fn get_highest_pid_without(&self, process_names: &[&str]) -> Option<Pid> {
        self.processes
            .iter()
            .filter(|(_, p)| {
                let p = p.lock();
                !process_names.iter().any(|n| p.get_name() == *n) && p.get_pid() != NEVER_PID
            })
            .max_by_key(|(pid, _)| *pid)
            .map(|(pid, _)| *pid)
    }

    pub fn dump(&self) {
        for (pid, process) in &self.processes {
            let process = process.lock();
            info!(
                "PID={} NAME={} STATE={:?}",
                *pid,
                process.get_name(),
                process.get_state()
            );
        }
    }

    pub fn kill(&mut self, pid: Pid) {
        assert!(
            pid != NEVER_PID,
            "We are not allowed to kill the never process"
        );
        debug!("Removing pid={pid} from process table");
        if let Some(process) = self.processes.remove(&pid) {
            assert_eq!(
                Arc::strong_count(&process),
                1,
                "There should no more than one process handles be active"
            );
            for pid in process.lock().get_notifies_on_die() {
                self.wake_process_up(*pid);
            }
        }
    }

    pub fn next_runnable(&self, old_pid: Pid) -> Option<ProcessRef> {
        let mut next_iter = self
            .processes
            .range(old_pid..)
            .skip(1)
            .filter_map(Self::filter_map_runnable_processes);

        if let Some(next_process) = next_iter.next() {
            Some(next_process.clone())
        } else {
            self.processes
                .iter()
                .filter_map(Self::filter_map_runnable_processes)
                .next()
                .cloned()
        }
    }

    fn filter_map_runnable_processes<'a>((_, p): (&Pid, &'a ProcessRef)) -> Option<&'a ProcessRef> {
        if p.lock().get_state() == ProcessState::Runnable {
            Some(p)
        } else {
            None
        }
    }

    pub fn get_process(&self, pid: Pid) -> Option<&ProcessRef> {
        self.processes.get(&pid)
    }

    pub fn get_dummy_process(&self) -> ProcessRef {
        self.processes
            .get(&NEVER_PID)
            .expect("The dummy process must always exist")
            .clone()
    }

    pub fn wake_process_up(&self, pid: Pid) {
        debug!("Waking process up with pid={pid}");
        let mut process = self.processes.get(&pid).expect("Process must exist").lock();
        assert_eq!(
            process.get_state(),
            ProcessState::Waiting,
            "Process must be in waiting state to be woken up"
        );
        process.set_state(ProcessState::Runnable);
    }
}
