// This variable contains the list of modules that should be logged. If "kernel" is specified, every module is logged.
const LOG_FOLLOWING_MODULES: &[&str] = &[];
const DONT_LOG_FOLLOWING_MODULES: &[&str] = &[
    "kernel::interrupts::trap",
    "kernel::debugging::unwinder",
    "kernel::debugging::symbols",
    "kernel::processes::scheduler",
    "kernel::processes::process_table",
    "kernel::processes::timer",
    "kernel::io::stdin_buf",
];

const fn const_starts_with(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.len() > h.len() {
        return false;
    }
    let mut i = 0;
    while i < n.len() {
        if h[i] != n[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub const fn should_log_module(module_name: &str) -> bool {
    let mut i = 0;
    while i < DONT_LOG_FOLLOWING_MODULES.len() {
        if const_starts_with(module_name, DONT_LOG_FOLLOWING_MODULES[i]) {
            return false;
        }
        i += 1;
    }
    i = 0;
    while i < LOG_FOLLOWING_MODULES.len() {
        if const_starts_with(module_name, LOG_FOLLOWING_MODULES[i]) {
            return true;
        }
        i += 1;
    }
    false
}
