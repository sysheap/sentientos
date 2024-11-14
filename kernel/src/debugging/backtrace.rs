use super::eh_frame_parser;
use crate::{
    assert::static_assert_size,
    debug,
    debugging::{
        eh_frame_parser::EhFrameParser,
        unwinder::{RegisterRule, Unwinder},
    },
    info,
    memory::linker_information::LinkerInformation,
};
use alloc::vec::Vec;
use common::mutex::Mutex;
// Needed for the native backtrace impl for debugging purposes
// use core::ffi::c_void;
// use unwinding::abi::{
//     UnwindContext, UnwindReasonCode, _Unwind_Backtrace, _Unwind_GetIP, with_context,
// };

#[allow(dead_code)]
#[derive(Debug)]
enum BacktraceNextError {
    RaIsZero,
    CouldNotGetFde(usize),
}

/// We keep the already parsed information in a Vec
/// even though we might not even need to produce a backtrace
/// But we want to avoid heap allocation while backtracing
/// in case of memory corruption.
struct Backtrace<'a> {
    fdes: Vec<eh_frame_parser::ParsedFDE<'a>>,
}

static BACKTRACE: Mutex<Backtrace> = Mutex::new(Backtrace::new());

impl<'a> Backtrace<'a> {
    const fn new() -> Self {
        Self { fdes: Vec::new() }
    }

    fn find(&self, pc: usize) -> Option<&eh_frame_parser::ParsedFDE<'a>> {
        self.fdes.iter().find(|&fde| fde.contains(pc))
    }

    fn init(&mut self) {
        assert!(self.fdes.is_empty(), "Init can only be called once.");

        let linker_information = LinkerInformation::new();
        let eh_frame_start = linker_information.eh_frame_section_start as *const u8;
        let eh_frame_size = linker_information.eh_frame_size;

        debug!(
            "eh frame at {:p} with size {:#x}",
            eh_frame_start, eh_frame_size
        );

        let eh_frame = unsafe { core::slice::from_raw_parts(eh_frame_start, eh_frame_size) };

        let eh_frame_parser = EhFrameParser::new(eh_frame);
        let eh_frames = eh_frame_parser.iter(linker_information.eh_frame_section_start);

        for frame in eh_frames {
            self.fdes.push(frame);
        }
    }

    fn next(&self, regs: &mut CallerSavedRegs) -> Result<usize, BacktraceNextError> {
        let ra = regs.ra();

        if ra == 0 {
            return Err(BacktraceNextError::RaIsZero);
        }

        // RA points to the next instruction. Move it back one byte such
        // that it points into the previous instruction.
        // This case must be handled different as soon as we have
        // signal trampolines.
        let fde = self
            .find(ra - 1)
            .ok_or(BacktraceNextError::CouldNotGetFde(ra))?;

        let unwinder = Unwinder::new(fde);

        let row = unwinder.find_row_for_address(ra);

        let cfa = regs[row.cfa_register as usize].wrapping_add(row.cfa_offset as usize);

        let mut new_regs = regs.clone();
        new_regs.set_sp(cfa);
        new_regs.set_ra(0);

        for (reg_index, rule) in row.register_rules.iter().enumerate() {
            let value = match rule {
                RegisterRule::None => {
                    continue;
                }
                RegisterRule::Offset(offset) => {
                    let ptr = (cfa.wrapping_add(*offset as usize)) as *const usize;
                    unsafe { ptr.read() }
                }
            };
            new_regs[reg_index] = value;
        }

        *regs = new_regs;

        Ok(ra)
    }
}

// We leave that here for debugging purposes
// I'm not entirely sure if my own backtrace implementation
// is fault free. But we will see that in the future.
// After multiple months of implementing this I'm done and want to move forward
// to something else.
// fn print_native() {
//     #[derive(Default)]
//     struct CallbackData {
//         counter: usize,
//     }

//     extern "C" fn callback(unwind_ctx: &UnwindContext<'_>, arg: *mut c_void) -> UnwindReasonCode {
//         let data = unsafe { &mut *(arg as *mut CallbackData) };
//         data.counter += 1;
//         info!("{}: {:#x}", data.counter, _Unwind_GetIP(unwind_ctx));
//         UnwindReasonCode::NO_REASON
//     }

//     let mut data = CallbackData::default();

//     _Unwind_Backtrace(callback, &mut data as *mut _ as _);
// }

/// You ask where I got the registers from? This is a good question.
/// I just looked what registers were mentioned in the eh_frame and added those.
/// Maybe there will be more in the future, then we have to add them.
/// I tried to generate the following code via a macro. However this is not possible,
/// because they won't allow to concatenate x$num_reg as a identifier and I need the
/// literal number to access it via an index.
#[derive(Debug, Clone, Default)]
struct CallerSavedRegs {
    x1: usize,
    x2: usize,
    x8: usize,
    x9: usize,
    x18: usize,
    x19: usize,
    x20: usize,
    x21: usize,
    x22: usize,
    x23: usize,
    x24: usize,
    x25: usize,
    x26: usize,
    x27: usize,
}

impl core::fmt::Display for CallerSavedRegs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        macro_rules! print_reg {
            ($reg:ident) => {
                writeln!(f, "{}: {:#x}", stringify!($reg), self.$reg)?
            };
        }

        print_reg!(x1);
        print_reg!(x2);
        print_reg!(x8);
        print_reg!(x9);
        print_reg!(x18);
        print_reg!(x19);
        print_reg!(x20);
        print_reg!(x21);
        print_reg!(x22);
        print_reg!(x23);
        print_reg!(x24);
        print_reg!(x25);
        print_reg!(x26);
        print_reg!(x27);

        Ok(())
    }
}

impl core::ops::Index<usize> for CallerSavedRegs {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            1 => &self.x1,
            2 => &self.x2,
            8 => &self.x8,
            9 => &self.x9,
            18 => &self.x18,
            19 => &self.x19,
            20 => &self.x20,
            21 => &self.x21,
            22 => &self.x22,
            23 => &self.x23,
            24 => &self.x24,
            25 => &self.x25,
            26 => &self.x26,
            27 => &self.x27,
            _ => panic!("Invalid index"),
        }
    }
}

impl core::ops::IndexMut<usize> for CallerSavedRegs {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            1 => &mut self.x1,
            2 => &mut self.x2,
            8 => &mut self.x8,
            9 => &mut self.x9,
            18 => &mut self.x18,
            19 => &mut self.x19,
            20 => &mut self.x20,
            21 => &mut self.x21,
            22 => &mut self.x22,
            23 => &mut self.x23,
            24 => &mut self.x24,
            25 => &mut self.x25,
            26 => &mut self.x26,
            27 => &mut self.x27,
            _ => panic!("Invalid index"),
        }
    }
}

// This value is referenced in the assembly of extern "C-unwind" fn dispatch
static_assert_size!(CallerSavedRegs, 0x70);

impl CallerSavedRegs {
    fn ra(&self) -> usize {
        self.x1
    }

    fn set_ra(&mut self, value: usize) {
        self.x1 = value;
    }

    fn set_sp(&mut self, value: usize) {
        self.x2 = value;
    }

    fn with_context<T>(f: extern "C" fn(&mut Self, &mut T), data: &mut T) {
        let mut regs = Self::default();

        dispatch(&mut regs, data, f);

        #[naked]
        extern "C-unwind" fn dispatch<T>(
            regs: &mut CallerSavedRegs,
            data: &mut T,
            f: extern "C" fn(&mut CallerSavedRegs, &mut T),
        ) {
            unsafe {
                core::arch::naked_asm!(
                    "
                     # regs is in a0
                     # data in a1
                     # f to call in a2
                     sd x1, 0x00(a0)   
                     sd x2, 0x08(a0)
                     sd x8, 0x10(a0)
                     sd x9, 0x18(a0)
                     sd x18, 0x20(a0)
                     sd x19, 0x28(a0)
                     sd x20, 0x30(a0)
                     sd x21, 0x38(a0)
                     sd x22, 0x40(a0)
                     sd x23, 0x48(a0)
                     sd x24, 0x50(a0)
                     sd x25, 0x58(a0)
                     sd x26, 0x60(a0)
                     sd x27, 0x68(a0)
                     # Save return address on stack
                     # It is important to change the stack
                     # pointer after the previous instructions
                     # Otherwise the wrong sp is saved (x2 == sp)
                     addi sp, sp, -0x08
                     sd ra, 0x00(sp)
                     jalr a2
                     ld ra, 0x00(sp)
                     addi sp, sp, 0x08
                     ret
                    "
                )
            }
        }
    }
}

pub fn init() {
    BACKTRACE.lock().init();
}

pub fn print() {
    CallerSavedRegs::with_context(unwind, &mut ());

    extern "C" fn unwind(regs: &mut CallerSavedRegs, _data: &mut ()) {
        let lock = BACKTRACE.lock();
        let mut counter = 0u64;
        loop {
            match lock.next(regs) {
                Ok(address) => {
                    info!("{counter}: {address:#x}");
                    counter += 1;
                }
                Err(BacktraceNextError::RaIsZero) => {
                    info!("{counter}: 0x0");
                    break;
                }
                Err(BacktraceNextError::CouldNotGetFde(address)) => {
                    // We don't have any backtracing info from here
                    // but anyways it is the end of our call stack
                    info!("{counter}: {address:#x}");
                    break;
                }
            }
        }
    }
}

#[cfg(not(miri))]
#[cfg(test)]
mod tests {
    use crate::debugging::backtrace::{Backtrace, BacktraceNextError, CallerSavedRegs};
    use alloc::collections::VecDeque;
    use core::ffi::c_void;
    use unwinding::abi::{UnwindContext, UnwindReasonCode, _Unwind_Backtrace, _Unwind_GetIP};

    #[test_case]
    fn backtrace() {
        #[derive(Default)]
        struct CallbackData {
            addresses: VecDeque<usize>,
        }

        extern "C" fn callback(
            unwind_ctx: &UnwindContext<'_>,
            arg: *mut c_void,
        ) -> UnwindReasonCode {
            let data = unsafe { &mut *(arg as *mut CallbackData) };
            data.addresses.push_back(_Unwind_GetIP(unwind_ctx));
            UnwindReasonCode::NO_REASON
        }

        let mut data = CallbackData::default();

        _Unwind_Backtrace(callback, &mut data as *mut _ as _);
        CallerSavedRegs::with_context(unwind, &mut data);

        extern "C" fn unwind(regs: &mut CallerSavedRegs, data: &mut CallbackData) {
            let mut backtrace = Backtrace::new();
            backtrace.init();

            let mut own_addr = VecDeque::new();

            loop {
                match backtrace.next(regs) {
                    Ok(address) => {
                        own_addr.push_back(address);
                    }
                    Err(BacktraceNextError::RaIsZero) => {
                        own_addr.push_back(0);
                        break;
                    }
                    Err(BacktraceNextError::CouldNotGetFde(address)) => {
                        own_addr.push_back(address as usize);
                        break;
                    }
                }
            }

            // Skip some items because they are inside the unwind functions itself
            data.addresses.pop_front();
            data.addresses.pop_front();
            own_addr.pop_front();

            assert_eq!(own_addr, data.addresses);
        }
    }
}
