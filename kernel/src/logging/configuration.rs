// This variable contains the list of modules that should be logged. If "kernel" is specified, every module is logged.
const LOG_FOLLOWING_MODULES: &[&str] = &[];

// TODO: This should be made compile-time, such that this thing doesn't need to be queried at runtime.
pub fn should_log_module(module_name: &str) -> bool {
    for &log_module in LOG_FOLLOWING_MODULES {
        if module_name.starts_with(log_module) {
            return true;
        }
    }
    false
}
