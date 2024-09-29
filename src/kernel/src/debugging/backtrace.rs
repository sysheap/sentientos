use core::{arch::asm, array::IntoIter};

use alloc::collections::BTreeMap;
use common::mutex::Mutex;

use crate::{
    debug,
    debugging::{
        eh_frame_parser::EhFrameParser,
        unwinder::{RegisterRule, Unwinder},
    },
    info,
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
        let eh_frame_start = linker_information.eh_frame_section_start as *const u8;
        let eh_frame_size = linker_information.eh_frame_size;

        debug!(
            "eh frame at {:p} with size {:#x}",
            eh_frame_start, eh_frame_size
        );

        let eh_frame = unsafe { core::slice::from_raw_parts(eh_frame_start, eh_frame_size) };

        let eh_frame_parser = EhFrameParser::new(eh_frame);
        let eh_frames = eh_frame_parser.iter(linker_information.eh_frame_section_start as u64);

        for frame in eh_frames {
            self.unwinwd_instructions
                .try_insert(frame.pc_begin, frame)
                .expect("There should not be an FDE in here with that address.");
        }
    }

    fn print(&self) {
        let mut regs = CallerSavedRegs::here();
        let pc = regs.pc.unwrap() as u64;
        let (_, fde) = self
            .unwinwd_instructions
            .range(..=pc)
            .next_back()
            .expect("Must exist");
        info!("pc={:#x} {:#x?}", pc, fde);
        let unwinder = Unwinder::new(fde);
        let row = unwinder.find_row_for_address(pc);

        let cfa = ((regs[row.cfa_register].unwrap() as i64) + row.cfa_offset) as u64;

        for reg_index in CallerSavedRegs::index_iter() {
            match row.register_rules[reg_index] {
                RegisterRule::Undef => {
                    regs[reg_index as u64] = None;
                }
                RegisterRule::Offset(offset) => {
                    let ptr = (cfa as i64 + offset) as u64 as *const usize;
                    let value = unsafe { ptr.read() };
                    regs[reg_index as u64] = Some(value);
                }
            }
        }

        todo!()
    }
}

/// You ask where I got the registers from? This is a good question.
/// I just looked what registers were mentioned in the eh_frame and added those.
/// Maybe there will be more in the future, then we have to add them.
/// I tried to generate the following code via a macro. However this is not possible,
/// because they won't allow to concatenate x$num_reg as a identifier and I need the
/// literal number to access it via an index.
#[derive(Default, Debug)]
struct CallerSavedRegs {
    pc: Option<usize>,
    x1: Option<usize>,
    x2: Option<usize>,
    x8: Option<usize>,
    x9: Option<usize>,
    x18: Option<usize>,
    x19: Option<usize>,
    x20: Option<usize>,
    x21: Option<usize>,
    x22: Option<usize>,
    x23: Option<usize>,
    x24: Option<usize>,
    x25: Option<usize>,
    x26: Option<usize>,
    x27: Option<usize>,
}

impl core::ops::Index<u64> for CallerSavedRegs {
    type Output = Option<usize>;

    fn index(&self, index: u64) -> &Self::Output {
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

impl core::ops::IndexMut<u64> for CallerSavedRegs {
    fn index_mut(&mut self, index: u64) -> &mut Self::Output {
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

impl CallerSavedRegs {
    fn index_iter() -> IntoIter<usize, 14> {
        [1, 2, 8, 9, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27].into_iter()
    }

    fn here() -> Self {
        let mut pc;
        let mut x1;
        let mut x2;
        let mut x8;
        let mut x9;
        let mut x18;
        let mut x19;
        let mut x20;
        let mut x21;
        let mut x22;
        let mut x23;
        let mut x24;
        let mut x25;
        let mut x26;
        let mut x27;

        unsafe {
            asm!(
                "auipc {}, 0", // Load current pc with offset 0
                "mv {}, x1",
                "mv {}, x2",
                "mv {}, x8",
                "mv {}, x9",
                "mv {}, x18",
                "mv {}, x19",
                "mv {}, x20",
                "mv {}, x21",
                "mv {}, x22",
                "mv {}, x23",
                "mv {}, x24",
                "mv {}, x25",
                "mv {}, x26",
                "mv {}, x27",
                out(reg) pc,
                out(reg) x1,
                out(reg) x2,
                out(reg) x8,
                out(reg) x9,
                out(reg) x18,
                out(reg) x19,
                out(reg) x20,
                out(reg) x21,
                out(reg) x22,
                out(reg) x23,
                out(reg) x24,
                out(reg) x25,
                out(reg) x26,
                out(reg) x27,
            );
        }
        // We want to have the ip value before the execution of the assembly block.
        // Therefore substract the instruction size.
        // pc -= 4;

        Self {
            pc: Some(pc),
            x1: Some(x1),
            x2: Some(x2),
            x8: Some(x8),
            x9: Some(x9),
            x18: Some(x18),
            x19: Some(x19),
            x20: Some(x20),
            x21: Some(x21),
            x22: Some(x22),
            x23: Some(x23),
            x24: Some(x24),
            x25: Some(x25),
            x26: Some(x26),
            x27: Some(x27),
        }
    }
}

pub fn init() {
    BACKTRACE.lock().init();
}

pub fn print() {
    BACKTRACE.lock().print();
}
