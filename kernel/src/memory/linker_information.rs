macro_rules! getter_address {
    ($name:ident) => {
        #[cfg(not(miri))]
        pub fn $name() -> usize {
            unsafe extern "C" {
                static ${concat(__, $name)}: usize;
            }
            core::ptr::addr_of!(${concat(__, $name)}) as usize
        }
        #[cfg(miri)]
        pub fn $name() -> usize {
            // When running under Miri we don't have any sections
            // Just choose any value which does not collide with any
            // other mappings
            common::util::align_down(u32::MAX as usize, $crate::memory::PAGE_SIZE)
        }
    };
}

macro_rules! getter {
    ($name:ident) => {
        getter_address!(${concat($name, _start)});
        getter_address!(${concat($name, _end)});
        pub fn ${concat($name, _size)}() -> usize {
            Self::${concat($name, _end)}() - Self::${concat($name, _start)}()
        }
    };
}

// Idea taken by https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html
macro_rules! count_idents {
    () => { 0 };
    ($first:ident $($rest:ident)*) => {1 + count_idents!($($rest)*)};
}

macro_rules! sections {
    ($(.$name:ident, $xwr:expr;)*) => {
        use $crate::memory::page_tables::MappingDescription;
        use $crate::memory::page_tables::XWRMode;

        pub struct LinkerInformation;

        impl LinkerInformation {
            $(getter!($name);)*

            // The heaps end address will be calcualted at runtime
            // Therefore, it is handled as a special case
            getter_address!(heap_start);

            #[cfg(not(miri))]
            pub fn all_mappings() -> [MappingDescription; count_idents!($($name)*)] {
                [
                    $(MappingDescription {
                      virtual_address_start: LinkerInformation::${concat($name, _start)}(),
                      size: LinkerInformation::${concat($name, _size)}(),
                      privileges: $xwr,
                      name: stringify!($name)
                    },)*
                ]
            }
            #[cfg(miri)]
            pub fn all_mappings() -> [MappingDescription; 0] {
                // When running under Miri we don't have any sections
                []
            }
        }
    };
}

sections! {
    .text, XWRMode::ReadExecute;
    .rodata, XWRMode::ReadOnly;
    .eh_frame, XWRMode::ReadOnly;
    .data, XWRMode::ReadWrite;
    .bss, XWRMode::ReadWrite;
    .kernel_stack, XWRMode::ReadWrite;
}
