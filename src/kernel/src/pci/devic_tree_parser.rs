use core::fmt::Debug;

use crate::device_tree::*;
use alloc::vec::Vec;
use common::big_endian::BigEndian;

#[derive(Debug)]
pub struct PCIInformation {
    pub pci_host_bridge_address: usize,
    pub pci_host_bridge_length: usize,
    pub ranges: Vec<PCIRange>,
}

impl PCIInformation {
    pub fn get_first_range_for_type(&self, space_code: u8) -> Option<&PCIRange> {
        self.ranges
            .iter()
            .find(|range| range.pci_bitfield.space_code() == space_code)
    }
}

#[repr(transparent)]
pub struct PCIBitField(u32);

impl PCIBitField {
    pub const CONFIGURATION_SPACE_CODE: u8 = 0b00;
    pub const IO_SPACE_CODE: u8 = 0b01;
    pub const MEMORY_SPACE_32_BIT_CODE: u8 = 0b10;
    pub const MEMORY_SPACE_64_BIT_CODE: u8 = 0b11;

    fn high_bits(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub fn relocatable(&self) -> bool {
        self.high_bits() & 0b1000_0000 != 0
    }

    pub fn prefetchable(&self) -> bool {
        self.high_bits() & 0b0100_0000 != 0
    }

    pub fn alias_address(&self) -> bool {
        self.high_bits() & 0b0010_0000 != 0
    }

    pub fn space_code(&self) -> u8 {
        self.high_bits() & 0b0000_0011
    }
}

impl Debug for PCIBitField {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let typ = match self.space_code() {
            Self::CONFIGURATION_SPACE_CODE => "configuration space",
            Self::IO_SPACE_CODE => "I/O space",
            Self::MEMORY_SPACE_32_BIT_CODE => "32 bit memory space",
            Self::MEMORY_SPACE_64_BIT_CODE => "64 bit memory space",
            _ => panic!("invalid space code"),
        };
        write!(
            f,
            "relocatable={}, prefetchable={}, aliased_address={}, space_code={}",
            self.relocatable(),
            self.prefetchable(),
            self.alias_address(),
            typ
        )
    }
}

impl From<u32> for PCIBitField {
    fn from(value: u32) -> Self {
        PCIBitField(value)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PCIRange {
    pub pci_bitfield: PCIBitField,
    pub pci_address: usize,
    pub cpu_address: usize,
    pub size: usize,
}

pub fn parse<'a>(dt_root_node: &'a Node<'a>) -> Option<PCIInformation> {
    let mut pci_information = PCIInformation {
        pci_host_bridge_address: 0,
        pci_host_bridge_length: 0,
        ranges: Vec::new(),
    };

    let NodeRefWithParentCellInfo {
        node,
        parent_adress_cell,
        parent_size_cell,
    } = dt_root_node.find_node("pci")?;

    let mut reg_property = node.get_property("reg")?;

    pci_information.pci_host_bridge_address = match parent_adress_cell? {
        1 => reg_property.consume_sized_type::<BigEndian<u32>>()?.get() as usize,
        2 => reg_property.consume_sized_type::<BigEndian<u64>>()?.get() as usize,
        _ => panic!("pci address cannot be larger than 64 bit"),
    };

    pci_information.pci_host_bridge_length = match parent_size_cell? {
        1 => reg_property.consume_sized_type::<BigEndian<u32>>()?.get() as usize,
        2 => reg_property.consume_sized_type::<BigEndian<u64>>()?.get() as usize,
        _ => panic!("pci size cannot be larger than 64 bit"),
    };

    let mut ranges_property = node.get_property("ranges")?;

    while !ranges_property.empty() {
        assert!(node.adress_cell? == 3, "pci addresses must be described by 3 u32 values: the bitfield and then the acutal address");
        let pci_bitfield = ranges_property
            .consume_sized_type::<BigEndian<u32>>()?
            .get();
        let pci_child_address = ranges_property
            .consume_sized_type::<BigEndian<u64>>()?
            .get() as usize;

        let parent_address = match parent_adress_cell? {
            1 => ranges_property
                .consume_sized_type::<BigEndian<u32>>()?
                .get() as usize,
            2 => ranges_property
                .consume_sized_type::<BigEndian<u64>>()?
                .get() as usize,
            _ => panic!("pci address cannot be larger than 64 bit"),
        };

        let size = match node.size_cell? {
            1 => ranges_property
                .consume_sized_type::<BigEndian<u32>>()?
                .get() as usize,
            2 => ranges_property
                .consume_sized_type::<BigEndian<u64>>()?
                .get() as usize,
            _ => panic!("pci size cannot be larger than 64 bit"),
        };

        pci_information.ranges.push(PCIRange {
            pci_bitfield: pci_bitfield.into(),
            pci_address: pci_child_address,
            cpu_address: parent_address,
            size,
        });
    }

    Some(pci_information)
}
