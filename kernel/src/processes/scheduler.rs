use alloc::collections::VecDeque;

use crate::klibc::{elf::ElfFile, Mutex};

use super::process::Process;

macro_rules! prog_bytes {
    ($prog_ident:ident, $prog_name:literal) => {
        const $prog_ident: &[u8] = include_bytes!(concat!(
            "../../../target/riscv64gc-unknown-none-elf/debug/",
            $prog_name
        ));
    };
}

prog_bytes!(PROG1, "prog1");
prog_bytes!(PROG2, "prog1");

const PROGRAMS: [&[u8]; 2] = [PROG1, PROG2];

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

pub fn schedule() {
    // TODO SCHEDULE
}
