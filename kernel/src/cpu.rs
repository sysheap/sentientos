use alloc::{boxed::Box, sync::Arc};
use common::syscalls::trap_frame::TrapFrame;
use core::{arch::asm, mem::offset_of, ptr::addr_of};

use crate::{
    klibc::{Spinlock, SpinlockGuard, runtime_initialized::RuntimeInitializedData, sizes::KiB},
    memory::page_tables::RootPageTableHolder,
    processes::{
        process::Process,
        scheduler::CpuScheduler,
        thread::{ThreadRef, ThreadWeakRef},
    },
    sbi::extensions::ipi_extension::sbi_send_ipi,
};

const KERNEL_STACK_SIZE: usize = KiB(512);

const SIE_STIE: usize = 5;
const SSTATUS_SPP: usize = 8;
const SIP_SSIP: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuId(usize);

impl CpuId {
    pub fn from_hart_id(hart_id: usize) -> Self {
        Self(hart_id)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for CpuId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub static STARTING_CPU_ID: RuntimeInitializedData<CpuId> = RuntimeInitializedData::new();

pub const TRAP_FRAME_OFFSET: usize = offset_of!(Cpu, trap_frame);

pub const KERNEL_PAGE_TABLES_SATP_OFFSET: usize = offset_of!(Cpu, kernel_page_tables_satp_value);

pub struct Cpu {
    kernel_page_tables_satp_value: usize,
    trap_frame: TrapFrame,
    scheduler: Spinlock<CpuScheduler>,
    cpu_id: CpuId,
    kernel_page_tables: RootPageTableHolder,
    number_cpus: usize,
}

macro_rules! read_csrr {
    ($name: ident) => {
        #[allow(dead_code)]
        pub fn ${concat(read_, $name)}() -> usize {
            if cfg!(miri) {
                return 0;
            }

            let $name: usize;
            // SAFETY: Reading a CSR has no memory side-effects; the value is
            // returned in a general-purpose register.
            unsafe {
                asm!(concat!("csrr {}, ", stringify!($name)), out(reg) $name);
            }
            $name
        }
    };
}

macro_rules! write_csrr {
    ($name: ident) => {
        #[allow(dead_code)]
        pub fn ${concat(write_, $name)}(value: usize)  {
            if cfg!(miri) {
                return ;
            }
            // SAFETY: Writing a CSR is a privileged operation with no memory
            // aliasing concerns. Callers are responsible for semantic correctness.
            unsafe {
                asm!(concat!("csrw ", stringify!($name), ", {}"), in(reg) value);
            }
        }

        #[allow(dead_code)]
        pub fn ${concat(csrs_, $name)}(mask: usize)  {
            if cfg!(miri) {
                return ;
            }
            // SAFETY: csrs (set bits) is a privileged CSR operation with no
            // memory aliasing concerns.
            unsafe {
                asm!(concat!("csrs ", stringify!($name), ", {}"), in(reg) mask);
            }
        }

        #[allow(dead_code)]
        pub fn ${concat(csrc_, $name)}(mask: usize)  {
            if cfg!(miri) {
                return ;
            }
            // SAFETY: csrc (clear bits) is a privileged CSR operation with no
            // memory aliasing concerns.
            unsafe {
                asm!(concat!("csrc ", stringify!($name), ", {}"), in(reg) mask);
            }
        }
    };
}

impl Cpu {
    read_csrr!(satp);
    read_csrr!(stval);
    read_csrr!(sepc);
    read_csrr!(scause);
    read_csrr!(sscratch);
    read_csrr!(sie);
    read_csrr!(sstatus);

    write_csrr!(satp);
    write_csrr!(sepc);
    write_csrr!(sscratch);
    write_csrr!(sstatus);
    write_csrr!(sie);
    write_csrr!(sip);

    pub fn ipi_to_all_but_me(&self) {
        assert!(
            self.number_cpus <= 64,
            "If we have more cpu's we need to use hart_mask_base, that is not implemented yet."
        );
        let mut mask = 0;
        for id in (0..self.number_cpus).filter(|i| *i != self.cpu_id.as_usize()) {
            mask |= 1 << id;
        }
        sbi_send_ipi(mask, 0).assert_success();
    }

    pub fn init(cpu_id: CpuId, number_cpus: usize) -> *const Cpu {
        assert!(
            cpu_id.as_usize() < number_cpus,
            "cpu_id {cpu_id} must be less than number_cpus {number_cpus}"
        );
        let kernel_stack = Box::leak(vec![0u8; KERNEL_STACK_SIZE].into_boxed_slice()).as_mut_ptr();
        let mut page_tables = RootPageTableHolder::new_with_kernel_mapping(true);

        let stack_start_virtual = (0usize).wrapping_sub(KERNEL_STACK_SIZE);

        page_tables.map(
            stack_start_virtual,
            kernel_stack as usize,
            KERNEL_STACK_SIZE,
            crate::memory::page_tables::XWRMode::ReadWrite,
            false,
            format!("KERNEL_STACK CPU {cpu_id}"),
        );

        let satp_value = page_tables.get_satp_value_from_page_tables();

        let cpu = Box::new(Self {
            kernel_page_tables_satp_value: satp_value,
            trap_frame: TrapFrame::zero(),
            scheduler: Spinlock::new(CpuScheduler::new()),
            cpu_id,
            number_cpus,
            kernel_page_tables: page_tables,
        });

        Box::leak(cpu) as *const Cpu
    }

    fn cpu_ptr() -> *mut Cpu {
        let ptr = Self::read_sscratch() as *mut Self;
        assert!(!ptr.is_null() && ptr.is_aligned());
        ptr
    }

    pub fn current() -> &'static Cpu {
        // SAFETY: The pointer points to a static and is therefore always valid.
        unsafe { &*Self::cpu_ptr() }
    }

    pub fn read_trap_frame() -> TrapFrame {
        let cpu_ptr = Self::cpu_ptr();
        // SAFETY: Cpu is statically allocated and offset
        // is calculated by the actual field offset.
        unsafe {
            let trap_frame_ptr = cpu_ptr.byte_add(TRAP_FRAME_OFFSET).cast::<TrapFrame>();
            trap_frame_ptr.read_volatile()
        }
    }

    pub fn write_trap_frame(trap_frame: TrapFrame) {
        let cpu_ptr = Self::cpu_ptr();
        // SAFETY: Cpu is statically allocated and offset
        // is calculated by the actual field offset.
        unsafe {
            let trap_frame_ptr = cpu_ptr.byte_add(TRAP_FRAME_OFFSET).cast::<TrapFrame>();
            trap_frame_ptr.write_volatile(trap_frame);
        }
    }

    pub fn with_scheduler<R>(f: impl FnOnce(SpinlockGuard<'_, CpuScheduler>) -> R) -> R {
        let cpu = Self::current();
        let scheduler = cpu.scheduler().lock();
        f(scheduler)
    }

    pub fn current_thread() -> ThreadRef {
        Self::with_scheduler(|s| s.get_current_thread().clone())
    }

    pub fn current_thread_weak() -> ThreadWeakRef {
        Self::with_scheduler(|s| Arc::downgrade(s.get_current_thread()))
    }

    pub fn with_current_process<R>(f: impl FnOnce(SpinlockGuard<'_, Process>) -> R) -> R {
        Self::with_scheduler(|s| f(s.get_current_process().lock()))
    }

    pub fn maybe_kernel_page_tables() -> Option<&'static RootPageTableHolder> {
        let ptr = Self::read_sscratch() as *mut Self;
        if ptr.is_null() || !ptr.is_aligned() {
            return None;
        }
        // SAFETY: We validate above that the pointer is non-null and aligned.
        // The Cpu struct is statically allocated via Box::leak and never freed.
        unsafe { Some(&(*ptr).kernel_page_tables) }
    }

    pub fn cpu_id() -> CpuId {
        let ptr = Self::read_sscratch() as *mut Self;
        if ptr.is_null() {
            return *STARTING_CPU_ID;
        }
        // SAFETY: The Cpu struct is statically allocated via Box::leak; addr_of!
        // reads the field without creating an intermediate reference.
        unsafe { *addr_of!((*ptr).cpu_id) }
    }

    pub fn activate_kernel_page_table(&self) {
        self.kernel_page_tables.activate_page_table();
    }

    pub fn scheduler(&self) -> &Spinlock<CpuScheduler> {
        &self.scheduler
    }

    /// # Safety
    /// Caller must ensure `satp_val` points to a valid page table.
    pub unsafe fn write_satp_and_fence(satp_val: usize) {
        Cpu::write_satp(satp_val);
        // SAFETY: sfence.vma flushes the TLB; required after changing satp.
        unsafe {
            asm!("sfence.vma");
        }
    }

    pub fn memory_fence() {
        // SAFETY: `fence` is a memory ordering instruction with no operands.
        unsafe {
            asm!("fence");
        }
    }

    /// # Safety
    /// Must only be called during panic or shutdown paths where no further
    /// interrupt handling is expected.
    pub unsafe fn disable_global_interrupts() {
        Self::csrc_sstatus(0b10);
        Self::write_sie(0);
    }

    pub fn wait_for_interrupt() {
        // SAFETY: `wfi` halts the hart until an interrupt arrives; it has
        // no memory side-effects.
        unsafe {
            asm!("wfi");
        }
    }

    #[allow(dead_code)]
    pub fn is_timer_enabled() -> bool {
        let sie = Self::read_sie();
        (sie & (1 << SIE_STIE)) > 0
    }

    pub fn enable_timer_interrupt() {
        Self::csrs_sie(1 << SIE_STIE);
    }

    /// Clear SSIP (supervisor software interrupt pending)
    pub fn clear_supervisor_software_interrupt() {
        Self::csrc_sip(1 << SIP_SSIP);
    }

    #[allow(dead_code)]
    pub fn is_in_kernel_mode() -> bool {
        let sstatus = Self::read_sstatus();
        (sstatus & (1 << SSTATUS_SPP)) > 0
    }

    pub fn set_ret_to_kernel_mode(kernel_mode: bool) {
        if kernel_mode {
            Self::csrs_sstatus(1 << SSTATUS_SPP);
        } else {
            Self::csrc_sstatus(1 << SSTATUS_SPP);
        }
    }
}

impl Drop for Cpu {
    fn drop(&mut self) {
        panic!("Cpu struct is never allowed to be dropped!");
    }
}
