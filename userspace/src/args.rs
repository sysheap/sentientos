use common::runtime_initialized::RuntimeInitializedData;

static ARGS_START: RuntimeInitializedData<*const u8> = RuntimeInitializedData::new();

pub fn init(args_start: *const u8) {
    ARGS_START.initialize(args_start);
}

pub fn args() -> Args {
    Args::new(*ARGS_START)
}

pub struct Args {
    current: *const u8,
}

impl Args {
    const fn new(current: *const u8) -> Self {
        Self { current }
    }
}

impl Iterator for Args {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        None
        // SAFTEY: We need to trust the kernel
        // let c_str = unsafe { std::ffi::CStr::from_ptr(self.current) };
        // let str = c_str
        //     .to_str()
        //     .expect("Kernel must give us only valid utf-8 chars");

        // if str.is_empty() {
        //     return None;
        // }

        // self.current = unsafe { self.current.add(str.len() + 1) };

        // Some(str)
    }
}
