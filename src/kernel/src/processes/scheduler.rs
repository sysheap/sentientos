use core::cell::RefCell;

use alloc::rc::Rc;
use common::mutex::Mutex;

use crate::klibc::elf::ElfFile;
use crate::klibc::macros::include_bytes_align_as;
use crate::memory::page_tables;
use crate::processes::process_list;
use crate::{cpu, debug};

use super::process::Process;
use super::process_list::add_process;

macro_rules! path_to_compiled_binaries {
    () => {
        "../../compiled_userspace/"
    };
}

macro_rules! prog_bytes {
    ($prog_ident:ident, $prog_name:literal) => {
        pub static $prog_ident: (&str, &[u8]) = (
            $prog_name,
            include_bytes_align_as!(u64, concat!(path_to_compiled_binaries!(), $prog_name)),
        );
    };
}

prog_bytes!(PROG1, "prog1");
prog_bytes!(PROG2, "prog2");
prog_bytes!(SHELL, "shell");

static PROGRAMS: [(&str, &[u8]); 3] = [PROG1, PROG2, SHELL];
static INIT_PROGRAM: &[u8] = SHELL.1;

pub fn initialize() {
    let elf = ElfFile::parse(INIT_PROGRAM).expect("Cannot parse ELF file");
    let process = Process::from_elf(&elf);
    add_process(process);
}

static CURRENT_PROCESS: Mutex<Option<Rc<RefCell<Process>>>> = Mutex::new(None);

extern "C" {
    fn restore_user_context() -> !;
}

pub fn schedule() {
    debug!("Schedule next process");
    prepare_next_process();
    unsafe {
        restore_user_context();
    }
}

pub fn kill_current_process() {
    CURRENT_PROCESS.lock().take();
    schedule();
}

pub fn schedule_program(name: &str) -> bool {
    for (prog_name, elf) in PROGRAMS {
        if name == prog_name {
            let elf = ElfFile::parse(elf).expect("Cannot parse ELF file");
            let process = Process::from_elf(&elf);
            add_process(process);
            return true;
        }
    }
    false
}

fn prepare_next_process() {
    let current_process = CURRENT_PROCESS.lock().take();

    if let Some(current_process) = current_process {
        current_process
            .borrow_mut()
            .set_program_counter(cpu::read_sepc());
        debug!("Saved context to current process");
        debug!("Current process: {:?}", current_process);
        process_list::enqueue(current_process);
    }

    let next_process_ref = process_list::next_runnable().expect("No processes left to schedule.");

    {
        let next_process = next_process_ref.borrow();

        let trap_frame_ptr = next_process.register_state_ptr();
        let pc = next_process.get_program_counter();
        let page_table = next_process.get_page_table();

        cpu::write_sscratch_register(trap_frame_ptr);
        cpu::write_sepc(pc);

        page_tables::activate_page_table(page_table);

        debug!("Next process: {:?}", next_process);
    }

    *CURRENT_PROCESS.lock() = Some(next_process_ref);
}
