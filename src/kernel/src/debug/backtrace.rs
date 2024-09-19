use core::arch::asm;

use alloc::collections::BTreeMap;
use common::mutex::Mutex;

use crate::{
    debug, debug::eh_frame_parser::EhFrameParser, info,
    memory::linker_information::LinkerInformation,
};

use super::eh_frame_parser;

/// We keep the already parsed information in a BTreeMap
/// even though we might not even need to produce a backtrace
/// But we want to avoid heap allocation while backtracing
/// in case of memory corruption.
struct Backtrace<'a> {
    unwinwd_instructions: BTreeMap<u64, eh_frame_parser::ParsedFDE<'a>>,
}

static BACKTRACE: Mutex<Backtrace> = Mutex::new(Backtrace::new());

impl<'a> Backtrace<'a> {
    const fn new() -> Self {
        Self {
            unwinwd_instructions: BTreeMap::new(),
        }
    }

    fn init(&mut self) {
        assert!(
            self.unwinwd_instructions.is_empty(),
            "Init can only be called once."
        );

        let linker_information = LinkerInformation::new();
        let eh_frame_start = linker_information.eh_frame_start as *const u8;
        let eh_frame_size = linker_information.eh_frame_size;

        debug!(
            "eh frame at {:p} with size {:#x}",
            eh_frame_start, eh_frame_size
        );

        let eh_frame = unsafe { core::slice::from_raw_parts(eh_frame_start, eh_frame_size) };

        let eh_frame_parser = EhFrameParser::new(eh_frame);
        let eh_frames = eh_frame_parser.iter(linker_information.eh_frame_start as u64);

        for frame in eh_frames {
            self.unwinwd_instructions
                .try_insert(frame.pc_begin, frame)
                .expect("There should not be an FDE in here with that address.");
        }
    }

    fn print(&self) {
        let regs = CallerSavedRegs::here();
        let ra = regs.ra as u64;
        let fde = self
            .unwinwd_instructions
            .range(..=ra)
            .next_back()
            .expect("Must exist");
        info!("ra={:#x} {:#x?}", ra, fde);
        todo!()
    }
}

#[derive(Default, Debug)]
struct CallerSavedRegs {
    ra: usize,
    t0: usize,
    t1: usize,
    t2: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
    t3: usize,
    t4: usize,
    t5: usize,
    t6: usize,
}

impl CallerSavedRegs {
    fn here() -> Self {
        let mut self_ = Self::default();

        unsafe {
            asm!(
                "mv {}, ra",
                "mv {}, t0",
                "mv {}, t1",
                "mv {}, t2",
                "mv {}, a0",
                "mv {}, a1",
                "mv {}, a2",
                "mv {}, a3",
                "mv {}, a4",
                "mv {}, a5",
                "mv {}, a6",
                "mv {}, a7",
                "mv {}, t3",
                "mv {}, t4",
                "mv {}, t5",
                "mv {}, t6",
                out(reg) self_.ra,
                out(reg) self_.t0,
                out(reg) self_.t1,
                out(reg) self_.t2,
                out(reg) self_.a0,
                out(reg) self_.a1,
                out(reg) self_.a2,
                out(reg) self_.a3,
                out(reg) self_.a4,
                out(reg) self_.a5,
                out(reg) self_.a6,
                out(reg) self_.a7,
                out(reg) self_.t3,
                out(reg) self_.t4,
                out(reg) self_.t5,
                out(reg) self_.t6,
            );
        }

        self_
    }
}

pub fn init() {
    BACKTRACE.lock().init();
}

pub fn print() {
    BACKTRACE.lock().print();
}
