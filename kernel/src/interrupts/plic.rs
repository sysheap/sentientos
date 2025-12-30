use common::runtime_initialized::RuntimeInitializedData;

use crate::{
    info,
    klibc::{MMIO, Spinlock},
};

pub const PLIC_BASE: usize = 0x0c00_0000;
pub const PLIC_SIZE: usize = 0x1000_0000;

pub struct Plic {
    priority_register_base: MMIO<u32>,
    // pending_register: MMIO<u32>,
    enable_register: MMIO<u32>,
    threshold_register: MMIO<u32>,
    claim_complete_register: MMIO<u32>,
}

impl Plic {
    fn new(plic_base: usize, hart_id: usize) -> Self {
        let context = hart_id * 2 + 1;
        // These constants are set to interrupt context 1 which corresponds to Supervisor Mode on Hart 0
        // If we support multiple harts, we will need to change these constants to be configurable
        Self {
            priority_register_base: MMIO::new(plic_base),
            // pending_register: MMIO::new(plic_base + 0x1000),
            enable_register: MMIO::new(plic_base + 0x2000 + (0x80 * context)),
            threshold_register: MMIO::new(plic_base + 0x20_0000 + (0x1000 * context)),
            claim_complete_register: MMIO::new(plic_base + 0x20_0004 + (0x1000 * context)),
        }
    }
    fn enable(&mut self, interrupt_id: u32) {
        self.enable_register |= 1 << interrupt_id;
    }

    fn set_priority(&mut self, interrupt_id: u32, priority: u32) {
        assert!(priority <= 7);
        unsafe {
            self.priority_register_base
                .add(interrupt_id as usize)
                .write(priority);
        }
    }

    fn set_threshold(&mut self, threshold: u32) {
        assert!(threshold <= 7);
        self.threshold_register.write(threshold);
    }

    pub fn get_next_pending(&mut self) -> Option<InterruptSource> {
        let open_interrupt = self.claim_complete_register.read();

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
        self.claim_complete_register.write(interrupt_id);
    }
}

pub static PLIC: RuntimeInitializedData<Spinlock<Plic>> = RuntimeInitializedData::new();

const UART_INTERRUPT_NUMBER: u32 = 10;

#[derive(PartialEq, Eq)]
pub enum InterruptSource {
    Uart,
    Else,
}

pub fn init_uart_interrupt(hart_id: usize) {
    info!("Initializing plic uart interrupt");

    PLIC.initialize(Spinlock::new(Plic::new(PLIC_BASE, hart_id)));

    let mut plic = PLIC.lock();
    plic.set_threshold(0);
    plic.enable(UART_INTERRUPT_NUMBER);
    plic.set_priority(UART_INTERRUPT_NUMBER, 1);
}
