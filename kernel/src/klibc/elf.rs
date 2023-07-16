use super::macros::static_assert_size;

const ELF_MAGIC_NUMBER: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

#[repr(u8)]
#[derive(PartialEq, Eq)]
enum BitFormat {
    Bit32 = 1,
    Bit64 = 2,
}

#[repr(u8)]
#[derive(PartialEq, Eq)]
enum Endianess {
    Little = 1,
    Big = 2,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[repr(u8)]
#[derive(PartialEq, Eq)]
enum OsAbi {
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
#[derive(PartialEq, Eq, Clone, Copy)]
enum FileType {
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
#[derive(PartialEq, Eq, Clone, Copy)]
enum Machine {
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
struct ElfHeader {
    magic_number: [u8; 4],
    bit_format: BitFormat,
    endianess: Endianess,
    version: u8,
    os_abi: OsAbi,
    abi_version: u8,
    padding: [u8; 7],
    object_file_type: FileType,
    machine: Machine,
    version2: u32,
    entry_point: u64,
    start_program_header: u64,
    start_of_section_header: u64,
    flags: u32,
    size_of_this_header: u16,
    size_program_header_entry: u16,
    number_of_entries_in_program_header: u16,
    size_section_header_entry: u16,
    number_of_entries_section_header: u16,
    index_of_section_names_in_section_header_table: u16,
}

static_assert_size!(ElfHeader, 64);

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
        let error = ElfFile::check_validity(data);

        if let Some(error) = error {
            Err(error)
        } else {
            Ok(Self { data })
        }
    }

    fn get_header(&self) -> &ElfHeader {
        assert!(self.data.len() >= core::mem::size_of::<ElfHeader>());
        // Safe because we only had out ElfFile if it is checked for consistency
        unsafe { &*(self.data.as_ptr() as *const ElfHeader) }
    }

    fn check_validity(data: &[u8]) -> Option<ElfParseErrors> {
        if data.len() < core::mem::size_of::<ElfHeader>() {
            return Some(ElfParseErrors::FileTooShort);
        }

        let header = unsafe { &*(data.as_ptr() as *const ElfHeader) };

        if header.magic_number != ELF_MAGIC_NUMBER {
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
mod test {
    use super::ElfFile;

    const TEST_ELF_FILE: &[u8] = include_bytes!("../test/test_data/elf/test.elf");

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
}
