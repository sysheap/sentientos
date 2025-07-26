#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[macro_export]
macro_rules! impl_binding {
    { fn $name:ident($($arg: ident: $type:ty),*) -> $ret:ty $body:block } => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name($($arg: $type),*) -> $ret $body
        // Typecheck
        const _: () = {
            type FnType = unsafe extern "C" fn($($type),*) -> $ret;
            const IMPLEMENTATION: FnType = $name;
            const HEADER: FnType = $crate::$name;
        };
    };
}
