use alloc::boxed::Box;
use alloc::collections::VecDeque;
use common::mutex::Mutex;

use crate::klibc::elf::ElfFile;
use crate::klibc::macros::include_bytes_align_as;
use crate::memory::page_tables;
use crate::{cpu, debug, info};

use super::process::Process;

macro_rules! path_to_compiled_binaries {
    () => {
        "../../compiled_userspace/"
    };
}

macro_rules! prog_bytes {
    ($prog_ident:ident, $prog_name:literal) => {
        pub static $prog_ident: &[u8] =
            include_bytes_align_as!(u64, concat!(path_to_compiled_binaries!(), $prog_name));
    };
}

prog_bytes!(PROG1, "prog1");
prog_bytes!(PROG2, "prog2");
prog_bytes!(SHELL, "shell");

static PROGRAMS: [&[u8]; 3] = [PROG1, PROG2, SHELL];
static INIT_PROGRAM: &[u8] = SHELL;

pub struct Scheduler {
    queue: VecDeque<Box<Process>>,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn initialize(&mut self) {
        info!("Initializing scheduler");

        // let elf = ElfFile::parse(INIT_PROGRAM).expect("Cannot parse ELF file");
        // let process = Process::from_elf(&elf);
        // self.queue.push_back(Box::new(process));
        for p in PROGRAMS.iter() {
            let elf = ElfFile::parse(p).expect("Cannot parse ELF file");
            let process = Process::from_elf(&elf);
            self.queue.push_back(Box::new(process));
        }
    }

    pub fn get_next(&mut self) -> Option<Box<Process>> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn enqueue(&mut self, process: Box<Process>) {
        self.queue.push_back(process);
    }
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
static CURRENT_PROCESS: Mutex<Option<Box<Process>>> = Mutex::new(None);

extern "C" {
    fn restore_user_context() -> !;
}

pub fn schedule() {
    debug!("Schedule next process");
    if prepare_next_process() {
        unsafe {
            restore_user_context();
        }
    } else {
        panic!("All processes died... ");
    }
}

pub fn kill_current_process() {
    {
        let mut current_process = CURRENT_PROCESS.lock();
        current_process.take();
    }
    schedule();
}

fn prepare_next_process() -> bool {
    let mut scheduler = SCHEDULER.lock();

    let mut current_process = CURRENT_PROCESS.lock();

    // No more processes to schedule
    if current_process.is_none() && scheduler.is_empty() {
        return false;
    }

    // Current process is the only process; We can skip the save and restore part
    if scheduler.is_empty() && current_process.is_some() {
        return true;
    }

    if let Some(ref mut current_process) = *current_process {
        current_process.set_program_counter(cpu::read_sepc());
        debug!("Saved context to current process");
        debug!("Current process: {:?}", current_process);
    }

    let next_process = scheduler.get_next().expect("No process to schedule!");

    let trap_frame_ptr = next_process.register_state_ptr();
    let pc = next_process.get_program_counter();
    let page_table = next_process.get_page_table();

    cpu::write_sscratch_register(trap_frame_ptr);
    cpu::write_sepc(pc);

    page_tables::activate_page_table(page_table);

    debug!("Next process: {:?}", next_process);

    let old_process = current_process.replace(next_process);

    if let Some(old_process) = old_process {
        scheduler.enqueue(old_process);
    }

    true
}
