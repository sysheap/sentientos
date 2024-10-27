use alloc::{collections::BTreeMap, vec::Vec};
use common::{big_endian::BigEndian, consumable_buffer::ConsumableBuffer, mutex::Mutex};
use core::{
    fmt::{Debug, Display},
    mem::size_of,
    slice,
};

use crate::debug;

// Use u64 for alignment purposes
static PARSED_DEVICE_TREE: Mutex<[u64; 8 * 1024]> = Mutex::new([0; 8 * 1024]);

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

    pub fn get_structure_block(&self) -> StructureBlockIterator {
        let offset = self.off_dt_struct.get();
        let start = self.offset_from_header(offset as usize);
        debug!("Structure Block Start: {:p}", start);
        StructureBlockIterator {
            buffer: ConsumableBuffer::new(unsafe {
                slice::from_raw_parts(start, self.size_dt_struct.get() as usize)
            }),
            header: self,
        }
    }

    fn get_string(&self, offset: usize) -> Option<&str> {
        let start: *const u8 = self.offset_from_header(self.off_dt_strings.get() as usize);
        let size = self.size_dt_strings.get() as usize;
        if offset >= size {
            return None;
        }
        let strings_data = unsafe { slice::from_raw_parts(start, size) };
        let mut consumable_buffer = ConsumableBuffer::new(&strings_data[offset..]);
        consumable_buffer.consume_str()
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

const FDT_BEGIN_NODE: u32 = 0x1;
const FDT_END_NODE: u32 = 0x2;
const FDT_PROP: u32 = 0x3;
const FDT_NOP: u32 = 0x4;
const FDT_END: u32 = 0x9;

#[derive(Debug)]
pub enum FdtToken<'a> {
    BeginNode(&'a str),
    EndNode,
    Prop(&'a str, ConsumableBuffer<'a>),
    Nop,
    End,
}

pub struct StructureBlockIterator<'a> {
    header: &'a Header,
    buffer: ConsumableBuffer<'a>,
}

impl<'a> Iterator for StructureBlockIterator<'a> {
    type Item = FdtToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.empty() {
            return None;
        }

        let numeric_token_value = self.buffer.consume_sized_type::<BigEndian<u32>>()?;
        let token = match numeric_token_value.get() {
            FDT_BEGIN_NODE => {
                let name = self.buffer.consume_str()?;
                self.buffer.consume_alignment(size_of::<u32>());
                FdtToken::BeginNode(name)
            }
            FDT_END_NODE => FdtToken::EndNode,
            FDT_PROP => {
                let len = self.buffer.consume_sized_type::<BigEndian<u32>>()?.get() as usize;
                let string_offset =
                    self.buffer.consume_sized_type::<BigEndian<u32>>()?.get() as usize;
                let data = self.buffer.consume_slice(len)?;
                self.buffer.consume_alignment(size_of::<u32>());
                let string = self.header.get_string(string_offset)?;
                FdtToken::Prop(string, ConsumableBuffer::new(data))
            }
            FDT_NOP => FdtToken::Nop,
            FDT_END => {
                assert!(self.buffer.empty());
                FdtToken::End
            }
            _ => panic!("Unknown token: {:#x}", numeric_token_value.get()),
        };

        Some(token)
    }
}

impl<'a> StructureBlockIterator<'a> {
    pub fn parse(self) -> Option<Node<'a>> {
        let mut node_stack = Vec::new();
        let fake_root = Node::new("");
        node_stack.push(fake_root);
        for node in self {
            match node {
                FdtToken::BeginNode(node_name) => {
                    node_stack.push(Node::new(node_name));
                }
                FdtToken::Prop(name, mut data) => {
                    let current_node = node_stack.last_mut()?;
                    if name == "#address-cells" {
                        let value = data.consume_sized_type::<BigEndian<u32>>()?;
                        current_node.adress_cell = Some(value.get());
                    }
                    if name == "#size-cells" {
                        let value = data.consume_sized_type::<BigEndian<u32>>()?;
                        current_node.size_cell = Some(value.get());
                    }
                    data.reset();
                    current_node.properties.insert(name, data);
                }
                FdtToken::EndNode => {
                    let current_node = node_stack.pop()?;
                    let parent = node_stack.last_mut()?;
                    parent.children.insert(current_node.name, current_node);
                }
                FdtToken::Nop => {}
                FdtToken::End => {
                    assert!(node_stack.len() == 1);
                }
            }
        }
        assert!(node_stack.len() == 1);
        let fake_root = node_stack.pop()?;
        assert!(fake_root.children.len() == 1);
        fake_root.children.into_values().next()
    }
}

#[derive(Debug)]
pub struct Node<'a> {
    name: &'a str,
    pub adress_cell: Option<u32>,
    pub size_cell: Option<u32>,
    properties: BTreeMap<&'a str, ConsumableBuffer<'a>>,
    children: BTreeMap<&'a str, Node<'a>>,
}

pub struct NodeRefWithParentCellInfo<'a> {
    pub node: &'a Node<'a>,
    pub parent_adress_cell: Option<u32>,
    pub parent_size_cell: Option<u32>,
}

impl<'a> Node<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name,
            properties: BTreeMap::new(),
            children: BTreeMap::new(),
            adress_cell: None,
            size_cell: None,
        }
    }

    pub fn get_property(&'a self, name: &'a str) -> Option<ConsumableBuffer<'a>> {
        self.properties
            .get(name)
            .map(|buffer| buffer.reset_and_clone())
    }

    pub fn find_node(&'a self, node_name: &'a str) -> Option<NodeRefWithParentCellInfo<'a>> {
        self.find_node_internal(node_name, None, None)
    }

    fn find_node_internal(
        &'a self,
        node_name: &'a str,
        parent_adress_cell: Option<u32>,
        parent_size_cell: Option<u32>,
    ) -> Option<NodeRefWithParentCellInfo<'a>> {
        debug!(
            "find_internal: current={} parent_adress={:?}, parent_size={:?}",
            self.name, parent_adress_cell, parent_size_cell
        );
        let current_name = match self.name.find('@') {
            Some(index) => &self.name[..index],
            None => self.name,
        };

        if current_name == node_name {
            return Some(NodeRefWithParentCellInfo {
                node: self,
                parent_adress_cell,
                parent_size_cell,
            });
        }
        for children in self.children.values() {
            let maybe_node =
                children.find_node_internal(node_name, self.adress_cell, self.size_cell);
            if maybe_node.is_some() {
                return maybe_node;
            }
        }
        None
    }
}

pub fn parse_and_copy(device_tree_pointer: *const ()) -> &'static Header {
    let header = unsafe { &*(device_tree_pointer as *const Header) };

    assert_eq!(header.magic.get(), FDT_MAGIC, "Device tree magic missmatch");
    assert_eq!(
        header.version.get(),
        FDT_VERSION,
        "Device tree version mismatch"
    );

    let size = header.totalsize.get() as usize;

    // SAFETY: We are the only thread that is running so accessing the static is safe
    let mut parsed_device_tree_lock = PARSED_DEVICE_TREE.lock();
    assert!(size <= parsed_device_tree_lock.len());
    unsafe {
        core::ptr::copy_nonoverlapping(
            device_tree_pointer as *const u8,
            parsed_device_tree_lock.as_mut_ptr() as *mut u8,
            size,
        );
        &*(parsed_device_tree_lock.as_ptr() as *const Header)
    }
}
