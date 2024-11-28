use crate::{
    assert::static_assert_size, debug, info, klibc::runtime_initialized::RuntimeInitializedData,
};
use common::{big_endian::BigEndian, consumable_buffer::ConsumableBuffer};
use core::{
    fmt::{Debug, Display},
    mem::size_of,
    ops::Range,
    slice,
};

const FDT_MAGIC: u32 = 0xd00dfeed;
const FDT_VERSION: u32 = 17;

pub static THE: RuntimeInitializedData<&'static DeviceTree> = RuntimeInitializedData::new();

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
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

static_assert_size!(Header, 40);

// It would have been nicer to have a struct which has the header
// as first field and then the rest of the data as a dynamic sized type.
// However, I learned through miri that the total size of a DST is rounded up
// to it's alignment. However, qemu does follow that. Therefore, it could be
// that the totalsize is not aligned by 4 byte which makes it UB to have a reference
// to such a struct. Therefore, let's take the ugly way and make the whole struct a
// DST.
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceTree {
    data: [u8],
}

impl DeviceTree {
    fn new(device_tree_pointer: *const ()) -> &'static Self {
        let magic = device_tree_pointer as *const BigEndian<u32>;
        assert!(magic.is_aligned(), "Device tree must be 4 byte aligned");
        assert!(!device_tree_pointer.is_null());
        // SAFETY: We need to read the magic value to determine if we really got a device tree
        let header = unsafe {
            assert_eq!(magic.read().get(), FDT_MAGIC);
            &*(device_tree_pointer as *const Header)
        };
        assert_eq!(
            header.version.get(),
            FDT_VERSION,
            "Device tree version mismatch"
        );
        // SAFETY: We validated the important fields above so this is probably a valid device tree
        unsafe {
            &*(core::ptr::from_raw_parts::<DeviceTree>(
                device_tree_pointer,
                // Metadata only describes the size of the last field
                header.totalsize.get() as usize,
            ))
        }
    }

    fn header(&self) -> &Header {
        unsafe { &*(self.data.as_ptr() as *const Header) }
    }

    fn offset_from_header<T>(&self, offset: usize) -> *const T {
        assert!(offset < self.header().totalsize.get() as usize);
        (self as *const DeviceTree).wrapping_byte_add(offset) as *const T
    }

    pub fn get_reserved_areas(&self) -> &[ReserveEntry] {
        let offset = self.header().off_mem_rsvmap.get();
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

    pub fn root_node(&self) -> Node {
        let offset = self.header().off_dt_struct.get();
        let start = self.offset_from_header(offset as usize);
        debug!("Structure Block Start: {:p}", start);
        // SAFETY: The data is provided by the firmware
        // we cannot really do more here to provide safety
        let data =
            unsafe { slice::from_raw_parts(start, self.header().size_dt_struct.get() as usize) };
        let structure_block = ConsumableBuffer::new(data);
        // The fake node is needed such address-cells and size-cells are porperly
        // parsed
        let fake_node = Node::new("fake_node", self, structure_block);
        fake_node
            .find_node("")
            .expect("There must be a unnamed root-node")
    }

    fn get_string(&self, offset: usize) -> Option<&str> {
        let start: *const u8 = self.offset_from_header(self.header().off_dt_strings.get() as usize);
        let size = self.header().size_dt_strings.get() as usize;
        if offset >= size {
            return None;
        }
        let strings_data = unsafe { slice::from_raw_parts(start, size) };
        let mut consumable_buffer = ConsumableBuffer::new(&strings_data[offset..]);
        consumable_buffer.consume_str()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ReserveEntry {
    address: u64,
    size: u64,
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

#[derive(Debug, PartialEq, Eq)]
pub enum FdtToken<'a> {
    BeginNode(&'a str),
    EndNode,
    Prop(&'a str, ConsumableBuffer<'a>),
    Nop,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<'a> {
    name: &'a str,
    pub address_cells: Option<u32>,
    pub size_cells: Option<u32>,
    pub parent_address_cells: Option<u32>,
    pub parent_size_cells: Option<u32>,
    device_tree: &'a DeviceTree,
    structure_block: ConsumableBuffer<'a>,
}

impl<'a> Node<'a> {
    fn new(
        name: &'a str,
        device_tree: &'a DeviceTree,
        structure_block: ConsumableBuffer<'a>,
    ) -> Self {
        let mut self_ = Self {
            name,
            device_tree,
            structure_block,
            address_cells: None,
            size_cells: None,
            parent_address_cells: None,
            parent_size_cells: None,
        };

        self_.address_cells = self_
            .get_property("#address-cells")
            .and_then(|mut b| b.consume_sized_type::<BigEndian<u32>>())
            .map(|be| be.get());
        self_.size_cells = self_
            .get_property("#size-cells")
            .and_then(|mut b| b.consume_sized_type::<BigEndian<u32>>())
            .map(|be| be.get());

        self_
    }

    pub fn find_node(&self, needle: &'a str) -> Option<Self> {
        let mut clone = self.clone();
        clone.find_node_recursive(needle)
    }

    fn find_node_recursive(&mut self, needle: &'a str) -> Option<Self> {
        if self.name.split('@').next() == Some(needle) {
            return Some(self.clone());
        }

        let mut parent_address_cell = None;
        let mut parent_size_cell = None;

        while let Some(token) = self.next() {
            match token {
                FdtToken::BeginNode(node_name) => {
                    let mut node =
                        Node::new(node_name, self.device_tree, self.structure_block.clone());
                    node.parent_address_cells = parent_address_cell;
                    node.parent_size_cells = parent_size_cell;
                    if let Some(target_node) = node.find_node_recursive(needle) {
                        return Some(target_node);
                    }
                    // Advance already parsed values
                    self.structure_block = node.structure_block;
                }
                FdtToken::Prop(prop, mut data) => {
                    if prop == "#address-cells" {
                        parent_address_cell =
                            Some(data.consume_sized_type::<BigEndian<u32>>()?.get());
                    }
                    if prop == "#size-cells" {
                        parent_size_cell = Some(data.consume_sized_type::<BigEndian<u32>>()?.get());
                    }
                }
                FdtToken::Nop => {}
                FdtToken::EndNode | FdtToken::End => {
                    return None;
                }
            }
        }

        None
    }

    pub fn get_property(&self, name: &'a str) -> Option<ConsumableBuffer<'a>> {
        for token in self {
            match token {
                FdtToken::Prop(prop_name, data) => {
                    if prop_name == name {
                        return Some(data);
                    }
                }
                FdtToken::Nop => {}
                // If we encounter any other token we already iterated through
                // all tokens here
                _ => break,
            }
        }
        None
    }

    pub fn parse_reg_property(&self) -> Option<Reg> {
        let mut reg_property = self.get_property("reg")?;
        let address = match self.parent_address_cells? {
            1 => reg_property.consume_sized_type::<BigEndian<u32>>()?.get() as usize,
            2 => reg_property.consume_sized_type::<BigEndian<u64>>()?.get() as usize,
            _ => panic!("address cannot be larger than 64 bit"),
        };

        let size = match self.parent_size_cells? {
            1 => reg_property.consume_sized_type::<BigEndian<u32>>()?.get() as usize,
            2 => reg_property.consume_sized_type::<BigEndian<u64>>()?.get() as usize,
            _ => panic!("size cannot be larger than 64 bit"),
        };

        Some(Reg { address, size })
    }
}

pub struct Reg {
    pub address: usize,
    pub size: usize,
}

impl<'a> IntoIterator for &Node<'a> {
    type Item = FdtToken<'a>;

    type IntoIter = Node<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.clone()
    }
}

impl<'a> Iterator for Node<'a> {
    type Item = FdtToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.structure_block.empty() {
            return None;
        }

        let numeric_token_value = self
            .structure_block
            .consume_sized_type::<BigEndian<u32>>()?;
        let token = match numeric_token_value.get() {
            FDT_BEGIN_NODE => {
                let name = self.structure_block.consume_str()?;
                self.structure_block.consume_alignment(size_of::<u32>());
                FdtToken::BeginNode(name)
            }
            FDT_END_NODE => FdtToken::EndNode,
            FDT_PROP => {
                let len = self
                    .structure_block
                    .consume_sized_type::<BigEndian<u32>>()?
                    .get() as usize;
                let string_offset = self
                    .structure_block
                    .consume_sized_type::<BigEndian<u32>>()?
                    .get() as usize;
                let data = self.structure_block.consume_slice(len)?;
                self.structure_block.consume_alignment(size_of::<u32>());
                let string = self.device_tree.get_string(string_offset)?;
                FdtToken::Prop(string, ConsumableBuffer::new(data))
            }
            FDT_NOP => FdtToken::Nop,
            FDT_END => {
                assert!(self.structure_block.empty());
                FdtToken::End
            }
            _ => panic!("Unknown token: {:#x}", numeric_token_value.get()),
        };

        Some(token)
    }
}

pub fn get_devicetree_range() -> Range<*const u8> {
    let size = THE.header().totalsize.get() as usize;
    let device_tree_pointer = *THE as *const DeviceTree as *const u8;

    device_tree_pointer..device_tree_pointer.wrapping_byte_add(size)
}

pub fn init(device_tree_pointer: *const ()) {
    info!("Initialize device tree at {device_tree_pointer:p}");
    let device_tree = DeviceTree::new(device_tree_pointer);
    assert!(
        device_tree.get_reserved_areas().is_empty(),
        "There should be no reserved memory regions"
    );
    THE.initialize(device_tree);
}

#[cfg(test)]
mod tests {
    use super::Node;
    use crate::{
        device_tree::{DeviceTree, Header},
        info,
        klibc::macros::include_bytes_align_as,
    };
    use common::big_endian::BigEndian;

    const DTB: &[u8] = include_bytes_align_as!(Header, "test/test_data/dtb");

    fn get_root_node() -> Node<'static> {
        let device_tree = DeviceTree::new(DTB.as_ptr() as *const ());
        assert!(device_tree.header().totalsize.get() as usize <= DTB.len());
        device_tree.root_node()
    }

    #[test_case]
    fn basic_values() {
        let root_node = get_root_node();

        assert_eq!(
            root_node
                .get_property("compatible")
                .and_then(|mut b| b.consume_str()),
            Some("riscv-virtio")
        );
        assert_eq!(
            root_node
                .get_property("model")
                .and_then(|mut b| b.consume_str()),
            Some("riscv-virtio,qemu")
        );
        assert_eq!(root_node.get_property("foobar"), None);
    }

    #[test_case]
    fn inexistent_node() {
        let root_node = get_root_node();
        assert!(root_node.find_node("foobar").is_none());
    }

    #[test_case]
    fn single_depth_node() {
        let root_node = get_root_node();

        let chosen = root_node
            .find_node("chosen")
            .expect("chosen node must exist");

        assert!(chosen.address_cells.is_none());
        assert!(chosen.size_cells.is_none());

        assert_eq!(chosen.parent_address_cells, root_node.address_cells);
        assert_eq!(chosen.parent_size_cells, root_node.size_cells);

        assert_eq!(
            chosen
                .get_property("rng-seed")
                .and_then(|mut b| b.consume_sized_type::<BigEndian<u32>>())
                .map(|big_endian| big_endian.get()),
            Some(0x6164a749)
        );
        assert_eq!(
            chosen
                .get_property("stdout-path")
                .and_then(|mut b| b.consume_str()),
            Some("/soc/serial@10000000")
        );
    }

    #[test_case]
    fn multiple_depth_node() {
        let root_node = get_root_node();

        let cpu0 = root_node.find_node("cpu").expect("cpu node must exist");

        assert_eq!(cpu0.name, "cpu@0");
        assert_eq!(cpu0.parent_address_cells, Some(1));
        assert_eq!(cpu0.parent_size_cells, Some(0));

        assert_eq!(
            cpu0.get_property("riscv,cboz-block-size")
                .and_then(|mut b| b.consume_sized_type::<BigEndian<u32>>())
                .map(|big_endian| big_endian.get()),
            Some(0x40)
        );

        assert!(
            cpu0.get_property("#interrupt-cells").is_none(),
            "Must not access nested nodes."
        );

        let interrupt_controller_cpu0 = cpu0
            .find_node("interrupt-controller")
            .expect("interrupt controller must be accessible.");
        let interrupt_controller_root_node = root_node
            .find_node("interrupt-controller")
            .expect("interrupt controller must be accessible.");

        assert_eq!(
            interrupt_controller_cpu0, interrupt_controller_root_node,
            "Node must be the same indepdendent where we got it from."
        );
    }

    #[test_case]
    fn cells() {
        let root_node = get_root_node();

        assert!(root_node.parent_address_cells.is_none());
        assert!(root_node.parent_size_cells.is_none());

        assert_eq!(root_node.address_cells, Some(2));
        assert_eq!(root_node.size_cells, Some(2));

        assert_cells("poweroff", Some(2), Some(2), None, None);
        assert_cells("platform-bus", Some(2), Some(2), Some(1), Some(1));
        assert_cells("memory", Some(2), Some(2), None, None);
        assert_cells("cpu", Some(1), Some(0), None, None);
        assert_cells("interrupt-controller", None, None, None, None);
    }

    fn assert_cells(
        node_name: &str,
        parent_address_cells: Option<u32>,
        parent_size_cells: Option<u32>,
        address_cells: Option<u32>,
        size_cells: Option<u32>,
    ) {
        let root_node = get_root_node();

        let node = root_node.find_node(node_name).expect("node must exist");

        assert_eq!(node.parent_address_cells, parent_address_cells);
        assert_eq!(node.parent_size_cells, parent_size_cells);

        assert_eq!(node.address_cells, address_cells);
        assert_eq!(node.size_cells, size_cells);
    }
}
