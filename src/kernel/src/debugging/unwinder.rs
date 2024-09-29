use crate::debug;

use super::eh_frame_parser::{Instruction, ParsedFDE};
use alloc::vec::Vec;

pub struct Unwinder<'a> {
    fde: &'a ParsedFDE<'a>,
    rows: Vec<Row>,
}

impl<'a> Unwinder<'a> {
    pub fn new(fde: &'a ParsedFDE<'a>) -> Self {
        let cde_instructions = &fde.cie.initial_instructions;
        let fde_instructions = &fde.instructions;
        let mut self_ = Self {
            fde,
            rows: vec![Row::new(fde.pc_begin)],
        };

        debug!("Evaluate cde instructions");
        self_.evaluate_instructions(cde_instructions);
        debug!("Evaluate fde instructions");
        self_.evaluate_instructions(fde_instructions);

        for row in &self_.rows {
            debug!("{row}");
        }

        let mut last_row = self_.last_row();
        last_row.end_address = fde.pc_begin + fde.address_range as u64;
        self_.update_last_row(&last_row);

        debug!("{} rows", self_.rows.len());

        self_
    }

    pub fn find_row_for_address(&self, address: u64) -> &Row {
        self.rows
            .iter()
            .find(|row| row.start_address == address)
            .expect("There must be an unwind rule.")
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    fn last_row(&self) -> Row {
        self.rows
            .last()
            .cloned()
            .expect("There must be always at least one row.")
    }

    fn update_last_row(&mut self, row: &Row) {
        *self
            .rows
            .last_mut()
            .expect("There must be always at least one row.") = row.clone();
    }

    fn evaluate_instructions(&mut self, instructions: &[Instruction]) {
        let mut current_address = self.fde.pc_begin;
        debug!("current address: {current_address}");
        let mut current_row = self.last_row();
        for instruction in instructions {
            match instruction {
                Instruction::AdvanceLoc { delta } => {
                    debug!("AdvanceLoc(delta={})", *delta);
                    current_address += *delta as u64;
                    current_row.end_address = current_address;
                    self.update_last_row(&current_row);
                    current_row.start_address = current_address;
                    current_row.end_address = 0;
                    self.rows.push(current_row.clone());
                    debug!("pushing new row with address {current_address}");
                }
                Instruction::Offset { register, offset } => {
                    debug!("Offset(register={}, offset={})", *register, *offset);
                    let real_offset =
                        (*offset as i64).wrapping_mul(self.fde.cie.data_alignment_factor);
                    current_row.register_rules[*register as usize] =
                        RegisterRule::Offset(real_offset);
                }
                Instruction::Restore { register } => {
                    debug!("Restre(register={})", *register);
                    let first_rule = self.rows.first().unwrap();
                    current_row.register_rules[*register as usize] =
                        first_rule.register_rules[*register as usize];
                }
                Instruction::DefCfa { register, offset } => {
                    debug!("DefCfa(register={}, offset={})", *register, *offset);
                    current_row.cfa_register = *register as u64;
                    current_row.cfa_offset = *offset as i64;
                }
                Instruction::DefCfaOffset { offset } => {
                    debug!("DefCfaOffset(offset={})", *offset);
                    current_row.cfa_offset = *offset as i64;
                }
                Instruction::Nop => {}
            }
        }
        self.update_last_row(&current_row);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Row {
    pub start_address: u64,
    pub end_address: u64,
    pub cfa_register: u64,
    pub cfa_offset: i64,
    pub register_rules: [RegisterRule; 32], // Not sure how many registers are defined; we will check that later
}

impl core::fmt::Display for Row {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Row[")?;
        writeln!(f, "address: {:}", self.start_address)?;
        writeln!(f, "cfa register: {:?}", self.cfa_register)?;
        writeln!(f, "cfa offset: {:?}", self.cfa_offset)?;
        writeln!(f, "register rules:")?;
        for (index, rule) in self
            .register_rules
            .iter()
            .filter(|&&r| r != RegisterRule::Undef)
            .enumerate()
        {
            writeln!(f, "\t{} {:?}", index, rule)?;
        }

        writeln!(f, "]")
    }
}

impl Row {
    fn new(address: u64) -> Self {
        Self {
            start_address: address,
            end_address: 0,
            cfa_register: 0,
            cfa_offset: 0,
            register_rules: [RegisterRule::Undef; 32],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterRule {
    Undef,
    Offset(i64),
}

#[cfg(test)]
impl PartialEq<gimli::RegisterRule<usize>> for RegisterRule {
    fn eq(&self, other: &gimli::RegisterRule<usize>) -> bool {
        match self {
            RegisterRule::Undef => matches!(other, gimli::RegisterRule::Undefined),
            RegisterRule::Offset(offset) => {
                matches!(other, gimli::RegisterRule::Offset(control_offset) if offset == control_offset)
            }
        }
    }
}
