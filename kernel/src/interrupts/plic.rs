use crate::{klibc::MMIO, println};

pub const PLIC_BASE: usize = 0x0c00_0000;
pub const PLIC_SIZE: usize = 0x3FFFFFC;

const PRIORITY_REGISTER_BASE: MMIO<u32> = MMIO::new(PLIC_BASE);
const PENDING_REGISTER: MMIO<u32> = MMIO::new(PLIC_BASE + 0x1000);
const ENABLE_REGISTER: MMIO<u32> = MMIO::new(PLIC_BASE + 0x2000);
const THRESHOLD_REGISTER: MMIO<u32> = MMIO::new(PLIC_BASE + 0x20_0000);
const CLAIM_COMPLETE_REGISTER: MMIO<u32> = MMIO::new(PLIC_BASE + 0x20_0004);

const UART_INTERRUPT_NUMBER: u32 = 10;

#[derive(PartialEq, Eq)]
pub enum InterruptSource {
    Uart,
    Else,
}

pub fn enable(interrupt_id: u32) {
    unsafe {
        ENABLE_REGISTER.write(ENABLE_REGISTER.read() | (1 << interrupt_id));
    }
}

pub fn set_priority(interrupt_id: u32, priority: u32) {
    assert!(priority <= 7);
    unsafe {
        PRIORITY_REGISTER_BASE
            .add(interrupt_id as usize)
            .write(priority);
    }
}

pub fn set_threshold(threshold: u32) {
    assert!(threshold <= 7);
    unsafe {
        THRESHOLD_REGISTER.write(threshold);
    }
}

pub fn get_next_pending() -> Option<InterruptSource> {
    let open_interrupt = unsafe { CLAIM_COMPLETE_REGISTER.read() };

    match open_interrupt {
        0 => None,
        UART_INTERRUPT_NUMBER => Some(InterruptSource::Uart),
        _ => Some(InterruptSource::Else),
    }
}

pub fn complete_interrupt(source: InterruptSource) {
    let interrupt_id = match source {
        InterruptSource::Uart => UART_INTERRUPT_NUMBER,
        InterruptSource::Else => panic!("Invalid interrupt source to complete."),
    };
    unsafe {
        CLAIM_COMPLETE_REGISTER.write(interrupt_id);
    }
}

pub fn init_uart_interrupt() {
    println!("Initializing plic uart interrupt");
    set_threshold(0);
    enable(UART_INTERRUPT_NUMBER);
    set_priority(UART_INTERRUPT_NUMBER, 1);
}
