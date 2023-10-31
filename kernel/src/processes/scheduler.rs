use alloc::collections::VecDeque;

use crate::cpu;
use crate::klibc::macros::include_bytes_align_as;
use crate::memory::page_tables;
use crate::{
    klibc::{elf::ElfFile, Mutex},
    println,
};

use super::process::Process;

#[cfg(debug_assertions)]
macro_rules! path_to_compiled_binaries {
    () => {
        "../../../target/riscv64gc-unknown-none-elf/debug/"
    };
}

#[cfg(not(debug_assertions))]
macro_rules! path_to_compiled_binaries {
    () => {
        "../../../target/riscv64gc-unknown-none-elf/release/"
    };
}

macro_rules! prog_bytes {
    ($prog_ident:ident, $prog_name:literal) => {
        pub static $prog_ident: &[u8] =
            include_bytes_align_as!(u64, concat!(path_to_compiled_binaries!(), $prog_name));
    };
}

prog_bytes!(PROG1, "prog1");
prog_bytes!(PROG2, "prog1");

static PROGRAMS: [&[u8]; 2] = [PROG1, PROG2];

pub struct Scheduler {
    queue: VecDeque<Process>,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn initialize(&mut self) {
        println!("Initializing scheduler");

        for progam in PROGRAMS {
            let elf = ElfFile::parse(progam).expect("Cannot parse ELF file");
            let process = Process::from_elf(&elf);
            self.queue.push_back(process);
        }
    }

    pub fn get_next(&mut self) -> Option<Process> {
        self.queue.pop_front()
    }

    pub fn enqueue(&mut self, process: Process) {
        self.queue.push_back(process);
    }
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
static CURRENT_PROCESS: Mutex<Option<Process>> = Mutex::new(None);

extern "C" {
    fn restore_user_context() -> !;
}

pub fn schedule() -> ! {
    prepare_next_process();
    unsafe {
        restore_user_context();
    }
}

fn prepare_next_process() {
    let mut scheduler = SCHEDULER.lock();
    let mut current_process = CURRENT_PROCESS.lock();

    assert!(current_process.is_none(), "Need to implement context save");

    let next_process = scheduler.get_next().expect("No process to schedule!");

    let trap_frame_ptr = next_process.register_state_ptr();
    let pc = next_process.get_program_counter();
    let page_table = next_process.get_page_table();

    cpu::write_sscratch_register(trap_frame_ptr);
    cpu::write_sepc_register(pc);

    page_tables::activate_page_table(page_table);

    current_process.replace(next_process);
}
