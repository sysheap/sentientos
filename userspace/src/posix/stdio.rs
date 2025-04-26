use core::ffi::CStr;

use crate::println;

const EOF: core::ffi::c_int = -1;

#[unsafe(no_mangle)]
pub extern "C" fn puts(str: *const core::ffi::c_char) -> core::ffi::c_int {
    let s = c_char_to_str(str);
    if let Some(s) = s {
        println!("{s}\n");
        s.len() as i32
    } else {
        EOF
    }
}

fn c_char_to_str(c_str: *const core::ffi::c_char) -> Option<&'static str> {
    if c_str.is_null() {
        return None;
    }

    // SAFETY: We're assuming the C string is valid and null-terminated.
    unsafe {
        CStr::from_ptr(c_str)
            .to_str() // Converts to a Rust &str
            .ok() // Returns `Some(&str)` or `None` in case of invalid UTF-8
    }
}
