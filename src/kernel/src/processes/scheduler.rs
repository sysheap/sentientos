use alloc::boxed::Box;
use alloc::collections::VecDeque;

use crate::klibc::macros::include_bytes_align_as;
use crate::klibc::{elf::ElfFile, Mutex};
use crate::memory::page_tables;
use crate::{cpu, debug, info};

use super::process::Process;

#[cfg(debug_assertions)]
macro_rules! path_to_compiled_binaries {
    () => {
        "../../compiled_userspace/bin/"
    };
}

#[cfg(not(debug_assertions))]
macro_rules! path_to_compiled_binaries {
    () => {
        "../../compiled_userspace/bin/"
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

static PROGRAMS: [&[u8]; 2] = [PROG1, PROG2];

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

        for progam in PROGRAMS {
            let elf = ElfFile::parse(progam).expect("Cannot parse ELF file");
            let process = Process::from_elf(&elf);
            self.queue.push_back(Box::new(process));
        }
    }

    pub fn get_next(&mut self) -> Option<Box<Process>> {
        self.queue.pop_front()
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

pub fn schedule() -> ! {
    debug!("Schedule next process");
    prepare_next_process();
    unsafe {
        restore_user_context();
    }
}

fn prepare_next_process() {
    let mut scheduler = SCHEDULER.lock();
    let mut current_process = CURRENT_PROCESS.lock();

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
}
