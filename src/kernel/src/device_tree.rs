use common::big_endian::BigEndian;
use core::{
    fmt::{Debug, Display},
    slice,
};

const FDT_MAGIC: u32 = 0xd00dfeed;
const FDT_VERSION: u32 = 17;

#[repr(C)]
pub struct Header {
    magic: BigEndian<u32>,
    totalsize: BigEndian<u32>,
    off_dt_struct: BigEndian<u32>,
    off_dt_strings: BigEndian<u32>,
    off_mem_rsvmap: BigEndian<u32>,
    version: BigEndian<u32>,
    last_comp_version: BigEndian<u32>,
    boot_cpuid_phys: BigEndian<u32>,
    size_dt_strings: BigEndian<u32>,
    size_dt_struct: BigEndian<u32>,
}

impl Header {
    fn offset_from_header<T>(&self, offset: usize) -> *const T {
        (self as *const Header).wrapping_byte_add(offset) as *const T
    }

    pub fn get_reserved_areas(&self) -> &[ReserveEntry] {
        let offset = self.off_mem_rsvmap.get();
        let start: *const ReserveEntry = self.offset_from_header(offset as usize);
        let mut len = 0;
        unsafe {
            loop {
                let entry = &*start.add(len);
                // The last entry is marked with address and size set to 0
                if entry.address == 0 && entry.size == 0 {
                    break;
                }
                len += 1;
            }
            slice::from_raw_parts(start, len)
        }
    }
}

impl Debug for Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Header")
            .field("magic", &format_args!("{:#x}", self.magic.get()))
            .field("totalsize", &format_args!("{:#x}", self.totalsize.get()))
            .field(
                "off_dt_struct",
                &format_args!("{:#x}", self.off_dt_struct.get()),
            )
            .field(
                "off_dt_strings",
                &format_args!("{:#x}", self.off_dt_strings.get()),
            )
            .field(
                "off_mem_rsvmap",
                &format_args!("{:#x}", self.off_mem_rsvmap.get()),
            )
            .field("version", &format_args!("{:#x}", self.version.get()))
            .field(
                "last_comp_version",
                &format_args!("{:#x}", self.last_comp_version.get()),
            )
            .field(
                "boot_cpuid_phys",
                &format_args!("{:#x}", self.boot_cpuid_phys.get()),
            )
            .field(
                "size_dt_strings",
                &format_args!("{:#x}", self.size_dt_strings.get()),
            )
            .field(
                "size_dt_struct",
                &format_args!("{:#x}", self.size_dt_struct.get()),
            )
            .finish()
    }
}

#[repr(C)]
pub struct ReserveEntry {
    address: u64,
    size: u64,
}

impl Debug for ReserveEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReserveEntry")
            .field("address", &format_args!("{:#x}", self.address))
            .field("size", &format_args!("{:#x}", self.size))
            .finish()
    }
}

impl Display for ReserveEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "RESERVED: {:#x} - {:#x} (size: {:#x})",
            self.address,
            self.address + self.size - 1,
            self.size
        )
    }
}

pub fn parse(device_tree_pointer: *const ()) -> &'static Header {
    let header = unsafe { &*(device_tree_pointer as *const Header) };

    assert_eq!(header.magic.get(), FDT_MAGIC, "Device tree magic missmatch");
    assert_eq!(
        header.version.get(),
        FDT_VERSION,
        "Device tree version mismatch"
    );

    header
}
