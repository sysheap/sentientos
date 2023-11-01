// This variable contains the list of modules that should be logged.
const LOG_FOLLOWING_MODULES: &[&str] = &["kernel"];

pub fn should_log_module(module_name: &str) -> bool {
    for &log_module in LOG_FOLLOWING_MODULES {
        if module_name.starts_with(log_module) {
            return true;
        }
    }
    false
}
