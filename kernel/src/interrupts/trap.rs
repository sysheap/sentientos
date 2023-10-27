use crate::{
    interrupts::plic::{self, InterruptSource},
    io::uart,
    print, println,
};

#[repr(packed)]
pub struct TrapFrame {
    registers: [usize; 32],
    floating_registers: [usize; 32],
}

impl TrapFrame {
    const STACK_POINTER_REGISTER_INDEX: usize = 2;

    pub const fn zero() -> Self {
        Self {
            registers: [0; 32],
            floating_registers: [0; 32],
        }
    }

    pub fn set_stack_pointer(&mut self, stack_pointer: usize) {
        self.registers[TrapFrame::STACK_POINTER_REGISTER_INDEX] = stack_pointer;
    }
}

#[repr(transparent)]
struct MCause(usize);

impl MCause {
    fn is_asynchronous(&self) -> bool {
        self.0 >> 63 == 1
    }

    fn get_exception_code(&self) -> usize {
        self.0 << 1 >> 1
    }

    fn get_reason(&self) -> &'static str {
        let is_asynchronous = self.is_asynchronous();

        if is_asynchronous {
            match self.get_exception_code() {
                0 => "Reserved",
                1 => "Supervisor software interrupt",
                2 => "Reserved",
                3 => "Machine software interrupt",
                4 => "Reserved",
                5 => "Supervisor timer interrupt",
                6 => "Reserved",
                7 => "Machine timer interrupt",
                8 => "Reserved",
                9 => "Supervisor external interrupt",
                10 => "Reserved",
                11 => "Machine external interrupt",
                12..=15 => "Reserved",
                _ => "Designated for platform use",
            }
        } else {
            match self.get_exception_code() {
                0 => "Instruction address misaligned",
                1 => "Instruction access fault",
                2 => "Illegal instruction",
                3 => "Breakpoint",
                4 => "Load address misaligned",
                5 => "Load access fault",
                6 => "Store/AMO address misaligned",
                7 => "Store/AMO access fault",
                8 => "Environment call from U-mode",
                9 => "Environment call from S-mode",
                10 => "Reserved",
                11 => "Environment call from M-Mode",
                12 => "Instruction page fault",
                13 => "Load page fault",
                14 => "Reserved",
                15 => "Store/AMO page fault",
                16..=23 => "Reserved",
                24..=31 => "Designated for custom use",
                32..=47 => "Reserved",
                48..=63 => "Designated for custom use",
                _ => "Reserved",
            }
        }
    }
}

#[no_mangle]
extern "C" fn machine_mode_trap(mcause: MCause, mtval: usize, mepc: usize, trap_fram: &TrapFrame) {
    if mcause.is_asynchronous() {
        println!(
            "Asynchronous Machine mode trap occurred! (mcause: {} (Reason: {})) (mtval: 0x{:x}) (mepc: 0x{:x})",
            mcause.get_exception_code(),
            mcause.get_reason(),
            mtval,
            mepc
        );
        match mcause.get_exception_code() {
            11 => {
                let plic_interrupt =
                    plic::get_next_pending().expect("There should be a pending interrupt.");
                assert!(plic_interrupt == InterruptSource::Uart);

                let input = uart::read().expect("There should be input from the uart.");

                match input {
                    8 => {
                        // This is a backspace, so we
                        // essentially have to write a space and
                        // backup again:
                        print!("{} {}", 8 as char, 8 as char);
                    }
                    10 | 13 => {
                        // Newline or carriage-return
                        println!();
                    }
                    _ => {
                        print!("{}", input as char);
                    }
                };

                plic::complete_interrupt(plic_interrupt);
            }
            _ => panic!("Inavlid external interrupt"),
        };
    } else {
        panic!(
            "Machine mode trap occurred! (mcause: {} (Reason: {})) (mtval: 0x{:x}) (mepc: 0x{:x})",
            mcause.get_exception_code(),
            mcause.get_reason(),
            mtval,
            mepc
        );
    }
}

#[no_mangle]
extern "C" fn supervisor_mode_trap(
    scause: MCause,
    stval: usize,
    sepc: usize,
    trap_frame: &TrapFrame,
) {
    panic!(
        "Supervisor mode trap occurred! (scause: {} (Reason: {})) (stval: 0x{:x})",
        scause.get_exception_code(),
        scause.get_reason(),
        stval
    );
}
