use common::mutex::Mutex;

use crate::{info, klibc::MMIO};

pub const PLIC_BASE: usize = 0x0c00_0000;
pub const PLIC_SIZE: usize = 0x1000_0000;

struct Plic {
    priority_register_base: MMIO<u32>,
    // pending_register: MMIO<u32>,
    enable_register: MMIO<u32>,
    threshold_register: MMIO<u32>,
    claim_complete_register: MMIO<u32>,
}

impl Plic {
    const unsafe fn new(plic_base: usize) -> Self {
        // These constants are set to interrupt context 1 which corresponds to Supervisor Mode on Hart 0
        // If we support multiple harts, we will need to change these constants to be configurable
        unsafe {
            Self {
                priority_register_base: MMIO::new(plic_base),
                // pending_register: MMIO::new(plic_base + 0x1000),
                enable_register: MMIO::new(plic_base + 0x2080),
                threshold_register: MMIO::new(plic_base + 0x20_1000),
                claim_complete_register: MMIO::new(plic_base + 0x20_1004),
            }
        }
    }
    pub fn enable(&mut self, interrupt_id: u32) {
        *self.enable_register |= 1 << interrupt_id;
    }

    pub fn set_priority(&mut self, interrupt_id: u32, priority: u32) {
        assert!(priority <= 7);
        unsafe {
            *self.priority_register_base.add(interrupt_id as usize) = priority;
        }
    }

    pub fn set_threshold(&mut self, threshold: u32) {
        assert!(threshold <= 7);
        *self.threshold_register = threshold;
    }

    pub fn get_next_pending(&mut self) -> Option<InterruptSource> {
        let open_interrupt = *self.claim_complete_register;

        match open_interrupt {
            0 => None,
            UART_INTERRUPT_NUMBER => Some(InterruptSource::Uart),
            _ => Some(InterruptSource::Else),
        }
    }

    pub fn complete_interrupt(&mut self, source: InterruptSource) {
        let interrupt_id = match source {
            InterruptSource::Uart => UART_INTERRUPT_NUMBER,
            InterruptSource::Else => panic!("Invalid interrupt source to complete."),
        };
        *self.claim_complete_register = interrupt_id;
    }
}

static PLIC: Mutex<Plic> = Mutex::new(unsafe { Plic::new(PLIC_BASE) });

const UART_INTERRUPT_NUMBER: u32 = 10;

#[derive(PartialEq, Eq)]
pub enum InterruptSource {
    Uart,
    Else,
}

pub fn init_uart_interrupt() {
    info!("Initializing plic uart interrupt");
    let mut plic = PLIC.lock();
    plic.set_threshold(0);
    plic.enable(UART_INTERRUPT_NUMBER);
    plic.set_priority(UART_INTERRUPT_NUMBER, 1);
}

pub fn get_next_pending() -> Option<InterruptSource> {
    PLIC.lock().get_next_pending()
}

pub fn complete_interrupt(source: InterruptSource) {
    PLIC.lock().complete_interrupt(source);
}
