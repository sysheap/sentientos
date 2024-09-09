use alloc::collections::BTreeMap;
use common::mutex::Mutex;

use crate::{
    debug, debug::eh_frame_parser::EhFrameParser, memory::linker_information::LinkerInformation,
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
        todo!()
    }
}

pub fn init() {
    BACKTRACE.lock().init();
}

pub fn print() {
    BACKTRACE.lock().print();
}
