use crate::device_tree::*;
use alloc::vec::Vec;
use common::big_endian::BigEndian;

#[derive(Debug)]
pub struct PCIInformation {
    pub pci_host_bridge_address: usize,
    pub pci_host_bridge_length: usize,
    ranges: Vec<PCIRange>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PCIRange {
    pci_bitfield: u32,
    pci_child_address: usize,
    parent_address: usize,
    size: usize,
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
            pci_bitfield,
            pci_child_address,
            parent_address,
            size,
        });
    }

    Some(pci_information)
}
