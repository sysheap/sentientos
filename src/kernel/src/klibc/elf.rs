use common::big_endian::BigEndian;

use crate::{assert::static_assert_size, debug};

const ELF_MAGIC_NUMBER: u32 = 0x7f454c46;

#[repr(u8)]
#[derive(PartialEq, Eq)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum BitFormat {
    Bit32 = 1,
    Bit64 = 2,
}

#[repr(u8)]
#[derive(PartialEq, Eq)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum Endianess {
    Little = 1,
    Big = 2,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[repr(u8)]
#[derive(PartialEq, Eq)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum OsAbi {
    SystemV = 0x0,
    HP_UX = 0x1,
    NetBSD = 0x2,
    Linux = 0x3,
    GNU_Hurd = 0x4,
    Solaris = 0x6,
    AIX = 0x7,
    IRIX = 0x8,
    FreeBSD = 0x9,
    Tru64 = 0xA,
    NovellModesto = 0xb,
    OpenBSD = 0xc,
    OpenVMS = 0xd,
    NonStop_Kernel = 0xe,
    AROS = 0x0f,
    FenixOS = 0x10,
    Nuxi_CloudABI = 0x11,
    Stratus_Tecnologies_OpenVOS = 0x12,
}

/// Warning: This only works for little endian at the moment.
#[allow(non_camel_case_types)]
#[repr(u16)]
#[derive(PartialEq, Eq)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum FileType {
    None = 0x0,
    RelocatableFile = 0x1,
    ExecutableFile = 0x2,
    SharedObject = 0x3,
    CoreFile = 0x4,
    ET_LOOS = 0xfe00,
    ET_HIOS = 0xfeff,
    ET_LOPROC = 0xff00,
    ET_HIPROC = 0xffff,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[repr(u16)]
#[derive(PartialEq, Eq)]
#[allow(dead_code)]
#[non_exhaustive]
pub enum Machine {
    NoSpecificInstructionSet = 0x0,
    AT_T_WE_32100 = 0x01,
    SPARC = 0x02,
    x86 = 0x03,
    Motorola_68000_M68k = 0x04,
    Motorola_88000_M88k = 0x05,
    Intel_MCU = 0x06,
    Intel_80860 = 0x07,
    MIPS = 0x08,
    IBM_System_370 = 0x09,
    MIPS_RS3000_Little_endian = 0x0A,
    Hewlett_Packard_PA_RISC = 0x0F,
    Intel_80960 = 0x13,
    PowerPC = 0x14,
    PowerPC_64_bit = 0x15,
    S390_including_S390x = 0x16,
    IBM_SPU_SPC = 0x17,
    NEC_V800 = 0x24,
    Fujitsu_FR20 = 0x25,
    TRW_RH_32 = 0x26,
    Motorola_RCE = 0x27,
    Arm_up_to_Armv7_AArch32 = 0x28,
    Digital_Alpha = 0x29,
    SuperH = 0x2A,
    SPARC_Version_9 = 0x2B,
    Siemens_TriCore_embedded_processor = 0x2C,
    Argonaut_RISC_Core = 0x2D,
    Hitachi_H8_300 = 0x2E,
    Hitachi_H8_300H = 0x2F,
    Hitachi_H8S = 0x30,
    Hitachi_H8_500 = 0x31,
    IA_64 = 0x32,
    Stanford_MIPS_X = 0x33,
    Motorola_ColdFire = 0x34,
    Motorola_M68HC12 = 0x35,
    Fujitsu_MMA_Multimedia_Accelerator = 0x36,
    Siemens_PCP = 0x37,
    Sony_nCPU_embedded_RISC_processor = 0x38,
    Denso_NDR1_microprocessor = 0x39,
    Motorola_Star_Core_processor = 0x3A,
    Toyota_ME16_processor = 0x3B,
    STMicroelectronics_ST100_processor = 0x3C,
    Advanced_Logic_Corp_TinyJ_embedded_processor_family = 0x3D,
    AMD_x86_64 = 0x3E,
    Sony_DSP_Processor = 0x3F,
    Digital_Equipment_Corp_PDP_10 = 0x40,
    Digital_Equipment_Corp_PDP_11 = 0x41,
    Siemens_FX66_microcontroller = 0x42,
    STMicroelectronics_ST9_8_16_bit_microcontroller = 0x43,
    STMicroelectronics_ST7_8_bit_microcontroller = 0x44,
    Motorola_MC68HC16_Microcontroller = 0x45,
    Motorola_MC68HC11_Microcontroller = 0x46,
    Motorola_MC68HC08_Microcontroller = 0x47,
    Motorola_MC68HC05_Microcontroller = 0x48,
    Silicon_Graphics_SVx = 0x49,
    STMicroelectronics_ST19_8_bit_microcontroller = 0x4A,
    Digital_VAX = 0x4B,
    Axis_Communications_32_bit_embedded_processor = 0x4C,
    Infineon_Technologies_32_bit_embedded_processor = 0x4D,
    Element_14_64_bit_DSP_Processor = 0x4E,
    LSI_Logic_16_bit_DSP_Processor = 0x4F,
    TMS320C6000_Family = 0x8C,
    MCST_Elbrus_e2k = 0xAF,
    Arm_64_bits_Armv8_AArch64 = 0xB7,
    Zilog_Z80 = 0xDC,
    RISC_V = 0xF3,
    Berkeley_Packet_Filter = 0xF7,
    WDC_65C816 = 0x101,
}

#[repr(C)]
pub struct ElfHeader {
    pub magic_number: BigEndian<u32>,
    pub bit_format: BitFormat,
    pub endianess: Endianess,
    pub version: u8,
    pub os_abi: OsAbi,
    pub abi_version: u8,
    pub padding: [u8; 7],
    pub object_file_type: FileType,
    pub machine: Machine,
    pub version2: u32,
    pub entry_point: u64,
    pub start_program_header: u64,
    pub start_of_section_header: u64,
    pub flags: u32,
    pub size_of_this_header: u16,
    pub size_program_header_entry: u16,
    pub number_of_entries_in_program_header: u16,
    pub size_section_header_entry: u16,
    pub number_of_entries_section_header: u16,
    pub index_of_section_names_in_section_header_table: u16,
}

static_assert_size!(ElfHeader, 64);

#[repr(u32)]
#[derive(Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum ProgramHeaderType {
    PT_NULL = 0x0,
    PT_LOAD = 0x1,
    PT_DYNAMIC = 0x2,
    PT_INTERP = 0x3,
    PT_NOTE = 0x4,
    PT_SLIB = 0x5,
    PT_PHDR = 0x6,
    PT_TLS = 0x7,
    PT_LOOS = 0x60000000,
    PT_HIOS = 0x6FFFFFFF,
    PT_LOPROC = 0x70000000,
    PT_HIPROC = 0x7FFFFFFF,
    GNU_STACK = 0x6474e551,
    RISCV_ATTRIBUT = 0x70000003,
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum ProgramHeaderFlags {
    X = 0x1,
    W = 0x2,
    WX = 0x3,
    R = 0x4,
    RX = 0x5,
    RW = 0x6,
    RWX = 0x7,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct ElfProgramHeaderEntry {
    pub header_type: ProgramHeaderType,
    pub access_flags: ProgramHeaderFlags,
    pub offset_in_file: u64,
    pub virtual_address: u64,
    pub physical_address: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub alignment: u64,
}

static_assert_size!(ElfProgramHeaderEntry, 0x38);

#[derive(Debug)]
pub enum ElfParseErrors {
    FileTooShort,
    MagicNumberWrong,
    Bit32IsNotSupported,
    BigEndianIsNotSupported,
    UnsupportedElfVersionNumber,
    UnsupportedOsABI,
    NotAnExceutableFile,
    NotAnRISCVExecutable,
}

pub struct ElfFile<'a> {
    data: &'a [u8],
}

impl<'a> ElfFile<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ElfParseErrors> {
        assert_eq!(data.as_ptr() as usize % 8, 0, "Elf file has to be aligned"); // TODO: Copy contents out of slice to create aligned structs

        let error = ElfFile::check_validity(data);

        if let Some(error) = error {
            Err(error)
        } else {
            Ok(Self { data })
        }
    }

    pub fn get_header(&self) -> &ElfHeader {
        assert!(self.data.len() >= core::mem::size_of::<ElfHeader>());
        // Safe because we only had out ElfFile if it is checked for consistency
        unsafe { &*(self.data.as_ptr() as *const ElfHeader) }
    }

    pub fn get_program_headers(&self) -> &[ElfProgramHeaderEntry] {
        let header = self.get_header();
        let number_of_entries = header.number_of_entries_in_program_header;
        let position_program_header = header.start_program_header;
        let entry_size = header.size_program_header_entry;

        assert_eq!(
            entry_size as usize,
            core::mem::size_of::<ElfProgramHeaderEntry>()
        );
        assert!(
            position_program_header as usize + (entry_size as usize * number_of_entries as usize)
                <= self.data.len()
        );

        let data = unsafe {
            let program_header_pointer = self
                .data
                .as_ptr()
                .byte_add(position_program_header as usize)
                as *const ElfProgramHeaderEntry;
            core::slice::from_raw_parts(program_header_pointer, number_of_entries as usize)
        };

        debug!("Program headers: {:#x?}", data);

        data
    }

    pub fn get_program_header_data(&self, program_header: &ElfProgramHeaderEntry) -> &[u8] {
        let start = program_header.offset_in_file as usize;
        let size = program_header.file_size as usize;

        &self.data[start..start + size]
    }

    fn check_validity(data: &[u8]) -> Option<ElfParseErrors> {
        if data.len() < core::mem::size_of::<ElfHeader>() {
            return Some(ElfParseErrors::FileTooShort);
        }

        let header = unsafe { &*(data.as_ptr() as *const ElfHeader) };

        if header.magic_number.get() != ELF_MAGIC_NUMBER {
            return Some(ElfParseErrors::MagicNumberWrong);
        }

        if header.bit_format != BitFormat::Bit64 {
            return Some(ElfParseErrors::Bit32IsNotSupported);
        }

        if header.endianess != Endianess::Little {
            return Some(ElfParseErrors::BigEndianIsNotSupported);
        }

        if header.version != 1 {
            return Some(ElfParseErrors::UnsupportedElfVersionNumber);
        }

        if header.os_abi != OsAbi::SystemV {
            return Some(ElfParseErrors::UnsupportedOsABI);
        }

        if header.object_file_type != FileType::ExecutableFile {
            return Some(ElfParseErrors::NotAnExceutableFile);
        }

        if header.machine != Machine::RISC_V {
            return Some(ElfParseErrors::NotAnRISCVExecutable);
        }

        if header.version2 != 1 {
            return Some(ElfParseErrors::UnsupportedElfVersionNumber);
        }

        let size_of_program_header =
            header.size_program_header_entry * header.number_of_entries_in_program_header;
        let end_of_program_header = header.start_program_header + size_of_program_header as u64;

        if end_of_program_header > data.len() as u64 {
            return Some(ElfParseErrors::FileTooShort);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::klibc::{elf::ProgramHeaderType, macros::include_bytes_align_as};

    use super::{ElfFile, ElfProgramHeaderEntry, ProgramHeaderFlags};

    static TEST_ELF_FILE: &[u8] = include_bytes_align_as!(u64, "../test/test_data/elf/test.elf");

    #[test_case]
    fn parse_basic_elf_file() {
        ElfFile::parse(TEST_ELF_FILE).expect("Elf file must be parsable");
    }

    #[test_case]
    fn check_values_of_elf_file() {
        let elf = ElfFile::parse(TEST_ELF_FILE).expect("Elf file must be parsable");
        let header = elf.get_header();

        assert_eq!(header.entry_point, 0x103c);
        assert_eq!(header.number_of_entries_in_program_header, 4);
        assert_eq!(header.number_of_entries_section_header, 20);
        assert_eq!(header.index_of_section_names_in_section_header_table, 18);
        assert_eq!(header.size_of_this_header, 64);
        assert_eq!(header.size_program_header_entry, 56);
        assert_eq!(header.size_section_header_entry, 64);
        assert_eq!(header.flags, 0x5);
    }

    #[test_case]
    fn check_program_header_values() {
        let elf = ElfFile::parse(TEST_ELF_FILE).expect("Elf file must be parsable");
        let program_headers = elf.get_program_headers();

        assert_eq!(program_headers.len(), 4);

        assert_eq!(
            program_headers[0],
            ElfProgramHeaderEntry {
                header_type: ProgramHeaderType::PT_LOAD,
                access_flags: ProgramHeaderFlags::RX,
                offset_in_file: 0x1000,
                virtual_address: 0x1000,
                physical_address: 0x1000,
                file_size: 0xba,
                memory_size: 0xba,
                alignment: 0x1000,
            }
        );

        assert_eq!(
            program_headers[1],
            ElfProgramHeaderEntry {
                header_type: ProgramHeaderType::PT_LOAD,
                access_flags: ProgramHeaderFlags::R,
                offset_in_file: 0x10c0,
                virtual_address: 0x10c0,
                physical_address: 0x10c0,
                file_size: 0xf0,
                memory_size: 0xf0,
                alignment: 0x1000,
            }
        );

        assert_eq!(
            program_headers[2],
            ElfProgramHeaderEntry {
                header_type: ProgramHeaderType::PT_LOAD,
                access_flags: ProgramHeaderFlags::RW,
                offset_in_file: 0x11b0,
                virtual_address: 0x11b0,
                physical_address: 0x11b0,
                file_size: 0x3a0,
                memory_size: 0x3a0,
                alignment: 0x1000,
            }
        );

        // The fourth element is GNU_STACK. We don't need it, therefore we don't check for it here.
    }
}
