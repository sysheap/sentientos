#![allow(dead_code)]

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use common::{
    consumable_buffer::ConsumableBuffer,
    leb128::{SignedLEB128, UnsignedLEB128},
};

/// This parser is far from complete (nor compliant probably)
/// The documentation of the eh_frame is very sparse and I did
/// some of the work here by looking into the gimli crate
/// But it should be enought to provide us backtrace information.
pub struct EhFrameParser<'a> {
    data: &'a [u8],
}

impl<'a> EhFrameParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn iter(&self, base_address: u64) -> EhFrameIterator<'a> {
        EhFrameIterator {
            data: ConsumableBuffer::new(self.data),
            parsed_cie: BTreeMap::new(),
            base_address,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedCIE<'a> {
    pub version: u8,
    pub augmentation_string: &'a str,
    pub address_size: u8,
    pub code_alignment_factor: u64,
    pub data_alignment_factor: i64,
    pub augmentation_data: Option<&'a [u8]>,
    pub return_address_register: u64,
    pub initial_instructions: Vec<Instruction>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedFDE<'a> {
    pub cie: Arc<ParsedCIE<'a>>,
    pub pc_begin: u64,
    pub address_range: u32,
    pub augmentation_data: Option<&'a [u8]>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Bitness {
    Bits64,
    Bits32,
}

pub struct EhFrameIterator<'a> {
    base_address: u64,
    data: ConsumableBuffer<'a>,
    parsed_cie: BTreeMap<usize, Arc<ParsedCIE<'a>>>,
}

impl<'a> Iterator for EhFrameIterator<'a> {
    type Item = ParsedFDE<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let offset_of_cie = self.data.position();
        let (mut length, bitness) = self.parse_length_and_bitness()?;
        assert_eq!(
            bitness,
            Bitness::Bits32,
            "Currently only 32 Bit DWARF Format is implemented."
        );

        let cie_offset_or_none = self.data.consume_sized_type::<u32>()? as usize;
        length -= 4;

        if cie_offset_or_none == 0 {
            self.parse_cie(length, offset_of_cie)?;
            self.next()
        } else {
            self.parse_fde(cie_offset_or_none, length)
        }
    }
}

impl<'a> EhFrameIterator<'a> {
    fn parse_cie(&mut self, length: usize, offset_of_cie: usize) -> Option<()> {
        let begin_position = self.data.position();
        let version = self.data.consume_sized_type::<u8>()?;
        assert_eq!(version, 1);
        let augmentation_string = self.data.consume_str()?;

        assert_eq!(augmentation_string, "zR");

        let _eh_data = if augmentation_string.contains("eh") {
            Some(self.data.consume_sized_type::<usize>()?)
        } else {
            None
        };

        let address_size = core::mem::size_of::<usize>() as u8;
        let code_alignment_factor = self.data.consume_unsized_type::<UnsignedLEB128>()?.get();
        let data_alignment_factor = self.data.consume_unsized_type::<SignedLEB128>()?.get();
        let return_address_register = self.data.consume_unsized_type::<UnsignedLEB128>()?.get();

        let augmentation_data = if augmentation_string.contains('z') {
            let length = self.data.consume_unsized_type::<UnsignedLEB128>()?.get();
            Some(self.data.consume_slice(length as usize)?)
        } else {
            None
        };

        let current_position = self.data.position();
        let size = current_position - begin_position;
        let initial_instructions = self.data.consume_slice(length - size)?;
        let initial_instructions = self.parse_instructions(initial_instructions);

        let parsed_cie = ParsedCIE {
            version,
            augmentation_string,
            address_size,
            code_alignment_factor,
            data_alignment_factor,
            augmentation_data,
            return_address_register,
            initial_instructions,
        };

        let arced_cie = Arc::new(parsed_cie);

        let already_exist = self.parsed_cie.insert(offset_of_cie, arced_cie);

        assert!(
            already_exist.is_none(),
            "There should be only one CIE at each offset value."
        );

        Some(())
    }

    fn parse_fde(&mut self, cie_offset: usize, length: usize) -> Option<ParsedFDE<'a>> {
        let fde_position = self.data.position();

        // We already read the CIE_POINTER field therefore, we need to remove 4 bytes
        let target_cie_location = fde_position - cie_offset - 4;

        // let real_cie_offset = begin_position - offset_cie;
        let cie = self
            .parsed_cie
            .get(&target_cie_location)
            .expect("The corresponding entry must exist.")
            .clone();

        // Parse encoding of PC Begin
        let pc_begin = if cie.augmentation_string.contains('R') {
            assert_eq!(cie.augmentation_data?.len(), 1);
            let augmentation_byte = cie.augmentation_data?[0];
            self.parse_pc_begin(augmentation_byte)?
        } else {
            self.data.consume_sized_type::<u32>()? as u64
        };

        let address_range = self.data.consume_sized_type::<u32>()?;

        let augmentation_data = if cie.augmentation_string.contains('z') {
            let length = self.data.consume_unsized_type::<UnsignedLEB128>()?.get();
            Some(self.data.consume_slice(length as usize)?)
        } else {
            None
        };

        // parse augmented data
        let current_position = self.data.position();
        let size = current_position - fde_position;
        let instructions = self.data.consume_slice(length - size)?;
        let instructions = self.parse_instructions(instructions);

        Some(ParsedFDE {
            cie,
            pc_begin,
            address_range,
            augmentation_data,
            instructions,
        })
    }

    fn parse_pc_begin(&mut self, augmentation_byte: u8) -> Option<u64> {
        #[allow(non_upper_case_globals)]
        const DW_EH_PE_sdata4: u8 = 0x0b;
        #[allow(non_upper_case_globals)]
        const DW_EH_PE_pcrel: u8 = 0x10;
        assert_eq!(
            augmentation_byte,
            DW_EH_PE_pcrel | DW_EH_PE_sdata4,
            "Implement more parsing variants for pc begin"
        );
        let current_offset = self.base_address + (self.data.position() as u64);
        let offset = self.data.consume_sized_type::<i32>()?;
        current_offset.checked_add_signed(offset as i64)
    }

    fn parse_length_and_bitness(&mut self) -> Option<(usize, Bitness)> {
        let length = self.data.consume_sized_type::<u32>()?;

        if length != 0xffffffff {
            return Some((length as usize, Bitness::Bits32));
        }

        let length = self.data.consume_sized_type::<u64>()?;
        Some((length as usize, Bitness::Bits64))
    }

    fn parse_instructions(&self, instructions: &[u8]) -> Vec<Instruction> {
        let mut instructions = ConsumableBuffer::new(instructions);
        let mut parsed_instructions = Vec::new();
        while let Some(parsed_instruction) = self.parse_instruction(&mut instructions) {
            parsed_instructions.push(parsed_instruction);
        }
        assert!(
            instructions.empty(),
            "We expect that all instructions should be parsed."
        );
        parsed_instructions
    }

    fn parse_instruction(&self, instructions: &mut ConsumableBuffer) -> Option<Instruction> {
        if instructions.empty() {
            return None;
        }
        let instruction = instructions.consume_sized_type::<u8>()?;

        let high_bits = instruction & consts::CFI_INSTRUCTION_HIGH_BITS_MASK;

        if high_bits == consts::DW_CFA_ADVANCE_LOC {
            let delta = instruction & consts::CFI_INSTRUCTION_LOW_BITS_MASK;
            return Some(Instruction::AdvanceLoc {
                delta: delta as u32,
            });
        }

        if high_bits == consts::DW_CFA_OFFSET {
            let register = (instruction & consts::CFI_INSTRUCTION_LOW_BITS_MASK) as u16;
            let offset = instructions.consume_unsized_type::<UnsignedLEB128>()?.get();
            return Some(Instruction::Offset { register, offset });
        }

        if high_bits == consts::DW_CFA_RESTORE {
            let register = (instruction & consts::CFI_INSTRUCTION_LOW_BITS_MASK) as u16;
            return Some(Instruction::Restore { register });
        }

        assert_eq!(high_bits, 0);

        match instruction {
            consts::DW_CFA_DEF_CFA => {
                let register =
                    u16::try_from(instructions.consume_unsized_type::<UnsignedLEB128>()?.get())
                        .unwrap();
                let offset = instructions.consume_unsized_type::<UnsignedLEB128>()?.get();
                Some(Instruction::DefCfa { register, offset })
            }
            consts::DW_CFA_DEF_CFA_OFFSET => {
                let offset = instructions.consume_unsized_type::<UnsignedLEB128>()?.get();
                Some(Instruction::DefCfaOffset { offset })
            }
            consts::DW_CFA_NOP => Some(Instruction::Nop),
            consts::DW_CFA_ADVANCE_LOC1 => {
                let delta = instructions.consume_sized_type::<u8>()?;
                Some(Instruction::AdvanceLoc {
                    delta: delta as u32,
                })
            }
            consts::DW_CFA_ADVANCE_LOC2 => {
                let delta = instructions.consume_sized_type::<u16>()?;
                Some(Instruction::AdvanceLoc {
                    delta: delta as u32,
                })
            }
            consts::DW_CFA_ADVANCE_LOC4 => {
                let delta = instructions.consume_sized_type::<u32>()?;
                Some(Instruction::AdvanceLoc { delta })
            }
            _ => panic!("Instruction {:#x} no implemented.", instruction),
        }
    }
}

mod consts {
    pub const CFI_INSTRUCTION_HIGH_BITS_MASK: u8 = 0b1100_0000;
    pub const CFI_INSTRUCTION_LOW_BITS_MASK: u8 = !CFI_INSTRUCTION_HIGH_BITS_MASK;
    pub const DW_CFA_ADVANCE_LOC: u8 = 0x01 << 6;
    pub const DW_CFA_OFFSET: u8 = 0x02 << 6;
    pub const DW_CFA_RESTORE: u8 = 0x03 << 6;
    pub const DW_CFA_DEF_CFA: u8 = 0x0c;
    pub const DW_CFA_DEF_CFA_OFFSET: u8 = 0x0e;
    pub const DW_CFA_NOP: u8 = 0;
    pub const DW_CFA_ADVANCE_LOC1: u8 = 0x02;
    pub const DW_CFA_ADVANCE_LOC2: u8 = 0x03;
    pub const DW_CFA_ADVANCE_LOC4: u8 = 0x04;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    AdvanceLoc { delta: u32 },
    Offset { register: u16, offset: u64 },
    Restore { register: u16 },
    DefCfa { register: u16, offset: u64 },
    DefCfaOffset { offset: u64 },
    Nop,
}

#[cfg(test)]
mod tests {

    use crate::{
        debug,
        debugging::{
            eh_frame_parser::EhFrameParser,
            unwinder::{Row, Unwinder},
        },
    };

    use super::{Instruction, ParsedCIE, ParsedFDE};

    use alloc::collections::BTreeMap;
    use elf::ElfBytes;
    use gimli::{
        constants, BaseAddresses, CallFrameInstruction, CallFrameInstructionIter,
        CommonInformationEntry, EhFrame, EndianSlice, FrameDescriptionEntry, LittleEndian,
        ReaderOffset, StoreOnHeap, UnwindContext, UnwindSection, UnwindTableRow,
    };

    const KERNEL_ELF_TEST_BINARY: &[u8] = include_bytes!("../test/test_data/elf/kernel");

    #[test_case]
    fn parser_works() {
        let file =
            ElfBytes::<elf::endian::LittleEndian>::minimal_parse(KERNEL_ELF_TEST_BINARY).unwrap();

        let eh_frame = file.section_header_by_name(".eh_frame").unwrap().unwrap();
        let text = file.section_header_by_name(".text").unwrap().unwrap();
        let (eh_frame_data, _) = file.section_data(&eh_frame).unwrap();

        let base_addresses = BaseAddresses::default()
            .set_eh_frame(eh_frame.sh_addr)
            .set_text(text.sh_addr);

        let control_eh_frame = EhFrame::new(eh_frame_data, gimli::LittleEndian);
        let mut control_entries = control_eh_frame.entries(&base_addresses);

        let mut control_cies = BTreeMap::new();

        let parser = EhFrameParser::new(eh_frame_data);
        let mut entries = parser.iter(eh_frame.sh_addr);

        while let Some(control_entry) = control_entries.next().unwrap() {
            match control_entry {
                gimli::CieOrFde::Cie(control_cie) => {
                    control_cies.insert(control_cie.offset(), control_cie);
                }
                gimli::CieOrFde::Fde(control_fde) => {
                    let parsed_fde = entries.next().unwrap();
                    let control_fde = control_fde
                        .parse(|_, _, _| {
                            control_cies
                                .get(&control_fde.cie_offset().0)
                                .cloned()
                                .ok_or(gimli::Error::Io)
                        })
                        .unwrap();

                    assert_eq!(control_fde, parsed_fde);

                    let insts = control_fde.instructions(&control_eh_frame, &base_addresses);
                    assert_same_instructions(&parsed_fde, insts);

                    // Evaluate the rows and check if they also match
                    let unwinder = Unwinder::new(&parsed_fde);
                    let mut parsed_rows = unwinder.rows().iter();

                    let mut ctx: UnwindContext<usize, StoreOnHeap> = UnwindContext::new_in();
                    let mut control_table = control_fde
                        .rows(&control_eh_frame, &base_addresses, &mut ctx)
                        .unwrap();

                    let mut counter = 0;
                    while let Some(control_row) = control_table.next_row().unwrap() {
                        let parsed_row = parsed_rows.next().unwrap();
                        assert_eq!(parsed_row, control_row);
                        counter += 1;
                        debug!("{counter} rows ok");
                    }
                }
            }
        }

        assert!(entries.next().is_none());
    }

    fn assert_same_instructions<'a>(
        fde: &ParsedFDE,
        mut control_instructions: CallFrameInstructionIter<EndianSlice<'a, LittleEndian>>,
    ) {
        let mut insts = fde.instructions.iter();
        while let Some(control_inst) = control_instructions.next().unwrap() {
            let inst = insts.next().unwrap();
            assert_eq!(control_inst, *inst);
        }
        assert!(insts.next().is_none());
    }

    impl<T: ReaderOffset> PartialEq<Instruction> for CallFrameInstruction<T> {
        fn eq(&self, other: &Instruction) -> bool {
            match other {
                Instruction::AdvanceLoc { delta: delta_ } => {
                    matches!(self, CallFrameInstruction::AdvanceLoc { delta } if delta_ == delta)
                }
                Instruction::Offset {
                    register: register_,
                    offset,
                } => matches!(
                self,
                CallFrameInstruction::Offset {
                    register,
                    factored_offset
                }
                if register.0 == *register_ && offset == factored_offset
                ),
                Instruction::Restore {
                    register: register_,
                } => {
                    matches!(self, CallFrameInstruction::Restore { register } if register.0 == *register_)
                }
                Instruction::DefCfa {
                    register: register_,
                    offset: offset_,
                } => {
                    matches!(self, CallFrameInstruction::DefCfa { register, offset } if register.0 == *register_ && offset == offset_)
                }
                Instruction::DefCfaOffset { offset: offset_ } => {
                    matches!(self, CallFrameInstruction::DefCfaOffset { offset } if offset == offset_)
                }
                Instruction::Nop => matches!(self, CallFrameInstruction::Nop),
            }
        }
    }

    impl<'a> PartialEq<ParsedFDE<'a>> for FrameDescriptionEntry<EndianSlice<'a, LittleEndian>, usize> {
        fn eq(&self, other: &ParsedFDE<'a>) -> bool {
            let ParsedFDE {
                cie,
                pc_begin,
                address_range,
                augmentation_data: _,
                instructions: _,
            } = other;
            *self.cie() == **cie
                && self.initial_address() == *pc_begin
                && self.len() == *address_range as u64
        }
    }

    impl<'a> PartialEq<ParsedCIE<'a>> for CommonInformationEntry<EndianSlice<'a, LittleEndian>, usize> {
        fn eq(&self, other: &ParsedCIE<'a>) -> bool {
            let ParsedCIE {
                version,
                augmentation_string,
                address_size,
                code_alignment_factor,
                data_alignment_factor,
                augmentation_data,
                return_address_register,
                initial_instructions: _,
            } = other;
            // Currently we only support zR enconding
            // So check if the control block has that too
            let augmentation_data = augmentation_data.unwrap();
            assert!(augmentation_data.len() == 1);
            let augmentation_ok = *augmentation_string == "zR"
                && !self.has_lsda()
                && self.personality().is_none()
                && !self.is_signal_trampoline()
                && (self.fde_address_encoding() == Some(constants::DwEhPe(augmentation_data[0])));
            // TODO: Parse instructions and compare
            self.version() == *version
                && self.address_size() == *address_size
                && self.code_alignment_factor() == *code_alignment_factor
                && self.data_alignment_factor() == *data_alignment_factor
                && self.return_address_register().0 as u64 == *return_address_register
                && augmentation_ok
        }
    }

    impl PartialEq<UnwindTableRow<usize>> for Row {
        fn eq(&self, other: &UnwindTableRow<usize>) -> bool {
            let (cfa_register, cfa_offset) = match other.cfa() {
                gimli::CfaRule::RegisterAndOffset { register, offset } => (register, offset),
                gimli::CfaRule::Expression(_) => panic!("Expressions are not supported."),
            };

            let metadata = self.start_address == other.start_address()
                && self.end_address == other.end_address()
                && self.cfa_register == cfa_register.0 as u64
                && self.cfa_offset == *cfa_offset;

            if !metadata {
                return false;
            }

            for (reg, rule) in other.registers() {
                if self.register_rules[reg.0 as usize] != *rule {
                    return false;
                }
            }
            true
        }
    }
}
