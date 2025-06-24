mod decode;

use alloc::vec::Vec;
use core::slice;

use decode::{FdtReserveEntry, Header};
use static_assertions::assert_cfg;

use crate::io::bytesreader::{read_null_term_str, BytesReader};

assert_cfg!(
    all(
        not(all(
            feature = "hardcode-devicetree",
            feature = "runtime-devicetree"
        )),
        any(
            feature = "hardcode-devicetree",
            feature = "runtime-devicetree"
        )
    ),
    "Must select one of hardcoded or runtime device tree loading"
);

#[repr(align(32))]
pub struct Dtb(&'static [u8]);

#[cfg(feature = "hardcode-devicetree")]
pub const DEVTREE: Dtb = Dtb(include_bytes!(env!("DEVTREE_PATH")));

impl Dtb {
    pub unsafe fn from_pointer(ptr: *const u8) -> Self {
        let size = *((ptr as *const u32).add(1));
        Dtb(slice::from_raw_parts(ptr, size.swap_bytes() as usize))
    }
}

#[derive(Debug)]
pub struct DeviceTree<'a> {
    header: Header,
    reserved: Vec<FdtReserveEntry>,
    root: DeviceTreeNode<'a>,
}

#[derive(Debug)]
pub struct DeviceTreeNode<'a> {
    name: &'a str,
    properties: Vec<(&'a str, &'a [u8])>,
    pub childeren: Vec<DeviceTreeNode<'a>>,
}

impl<'a> DeviceTree<'a> {
    pub fn get_nodes(&self, path: &str) -> Vec<&DeviceTreeNode<'a>> {
        let path = path.strip_prefix('/').unwrap();

        self.root.get_childeren(path)
    }

    pub fn regs_for_node(
        &self,
        path: &str,
        unit_addr: Option<&str>,
    ) -> Option<Vec<(usize, usize)>> {
        self.root
            .regs_for_node_int(path.strip_prefix('/').unwrap(), unit_addr, 2, 2)
    }
}

impl<'a> DeviceTreeNode<'a> {
    pub fn get_and_parse_property<T, F: Fn(&[u8]) -> T>(
        &self,
        property: &str,
        parse: F,
    ) -> Option<T> {
        self.properties
            .iter()
            .find(|(n, _)| *n == property)
            .map(|(_, v)| parse(v))
    }

    pub fn address_size_cells(&self) -> (Option<u32>, Option<u32>) {
        fn parse_u32(b: &[u8]) -> u32 {
            let mut buf = [0; 4];
            buf.copy_from_slice(&b[0..4]);
            u32::from_be_bytes(buf)
        }

        let address_cells = self.get_and_parse_property("#address_cells", parse_u32);

        let size_cells = self.get_and_parse_property("#size_cells", parse_u32);

        (address_cells, size_cells)
    }

    pub fn get_childeren(&self, path: &str) -> Vec<&DeviceTreeNode<'a>> {
        if let Some((name, child_path)) = path.split_once('/') {
            self.childeren
                .iter()
                .find(|c| {
                    if let Some((child_name, _)) = c.name.split_once('@') {
                        child_name == name
                    } else {
                        c.name == name
                    }
                })
                .map(|c| c.get_childeren(child_path))
                .unwrap_or(Vec::new())
        } else {
            let mut childeren = Vec::new();
            self.childeren
                .iter()
                .filter(|c| {
                    if let Some((child_name, _)) = c.name.split_once('@') {
                        child_name == path
                    } else {
                        c.name == path
                    }
                })
                .collect_into(&mut childeren);

            childeren
        }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn unit_name(&self) -> &str {
        self.name
            .split_once('@')
            .map(|(n, _)| n)
            .unwrap_or(self.name)
    }

    pub fn unit_addr(&self) -> Option<&str> {
        self.name.split_once('@').map(|(_, a)| a)
    }

    fn regs_for_node_int(
        &self,
        path: &str,
        unit_addr: Option<&str>,
        default_addr_cells: u32,
        default_size_cells: u32,
    ) -> Option<Vec<(usize, usize)>> {
        let (addr_cells, size_cells) = self.address_size_cells();
        let (addr_cells, size_cells) = (
            addr_cells.unwrap_or(default_addr_cells),
            size_cells.unwrap_or(default_size_cells),
        );

        if let Some((name, child_path)) = path.split_once('/') {
            self.childeren
                .iter()
                .find(|c| {
                    if let Some((child_name, _)) = c.name.split_once('@') {
                        child_name == name
                    } else {
                        c.name == name
                    }
                })
                .and_then(|c| c.regs_for_node_int(child_path, unit_addr, addr_cells, size_cells))
        } else {
            let parse_reg = |b: &[u8]| {
                let mut pairs = Vec::new();
                b.chunks(addr_cells as usize * 4 + size_cells as usize * 4)
                    .map(|c| {
                        let mut reader = BytesReader::new(c);

                        let mut addr = 0usize;
                        for i in 1..=addr_cells {
                            let cell = reader.read_u32_be() as usize;
                            addr |= cell << (32 * (addr_cells - i));
                        }

                        let mut size = 0usize;
                        for i in 1..=size_cells {
                            let cell = reader.read_u32_be() as usize;
                            size |= cell << (32 * (size_cells - i));
                        }

                        (addr, size)
                    })
                    .collect_into(&mut pairs);
                pairs
            };
            if let Some(addr) = unit_addr {
                self.childeren
                    .iter()
                    .find(|c| {
                        if let Some((child_name, child_addr)) = c.name.split_once('@') {
                            child_name == path && child_addr == addr
                        } else {
                            false
                        }
                    })
                    .and_then(|n| n.get_and_parse_property("reg", parse_reg))
            } else {
                self.childeren
                    .iter()
                    .find(|c| c.name == path)
                    .and_then(|n| n.get_and_parse_property("reg", parse_reg))
            }
        }
    }
}

pub fn decode_dtb<'a, 'b: 'a>(dtb: &'b Dtb) -> DeviceTree<'a> {
    let mut reader = BytesReader::new(dtb.0);

    let header = Header {
        magic: reader.read_u32_be(),
        totalsize: reader.read_u32_be(),
        off_dt_struct: reader.read_u32_be(),
        off_dt_string: reader.read_u32_be(),
        off_mem_rsvmap: reader.read_u32_be(),
        version: reader.read_u32_be(),
        last_comp_version: reader.read_u32_be(),
        boot_cpuid_phys: reader.read_u32_be(),
        size_dt_strings: reader.read_u32_be(),
        size_dt_struct: reader.read_u32_be(),
    };

    assert_eq!(
        dtb.0.len(),
        header.totalsize as usize,
        "Device tree has a different size than reported"
    );
    assert_eq!(header.version, 17);

    let mut reader = BytesReader::new(&dtb.0[(header.off_mem_rsvmap as usize)..]);

    let mut reserved = Vec::new();

    while let Some(entry) = read_fdt_reserve_entry(&mut reader) {
        reserved.push(entry);
    }

    let mut nodes = BytesReader::new(
        &dtb.0[(header.off_dt_struct as usize)
            ..(header.off_dt_struct as usize + header.size_dt_struct as usize)],
    );
    let strings = &dtb.0[(header.off_dt_string as usize)
        ..(header.off_dt_string as usize + header.size_dt_strings as usize)];

    let root = read_node(unsafe { &mut *(&mut nodes as *mut BytesReader) }, strings);

    DeviceTree {
        header,
        reserved,
        root,
    }
}

fn read_fdt_reserve_entry(entries: &mut BytesReader) -> Option<FdtReserveEntry> {
    let addr = entries.read_u64_be();
    let size = entries.read_u64_be();
    if addr != 0 && size != 0 {
        Some(FdtReserveEntry { addr, size })
    } else {
        None
    }
}

const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009;

fn read_node<'a>(nodes: &'a mut BytesReader, strings: &'a [u8]) -> DeviceTreeNode<'a> {
    while nodes.peek_u32_be() == FDT_NOP {
        nodes.read_u64_be();
    }

    let token = nodes.read_u32_be();
    assert_eq!(token, FDT_BEGIN_NODE);
    let name = nodes.null_term_str();
    nodes.align_to(4);

    let mut properties = Vec::new();
    let mut childeren = Vec::new();

    while matches!(
        nodes.peek_u32_be(),
        FDT_BEGIN_NODE | FDT_END_NODE | FDT_PROP | FDT_NOP
    ) {
        match nodes.peek_u32_be() {
            FDT_BEGIN_NODE => childeren.push(read_node(
                // Fuck the borrow checker, this is fine because we only return references to the
                // read only data, there does not exist a reference to the mutable part after the
                // return or read_node
                unsafe { &mut *(nodes as *mut BytesReader) },
                strings,
            )),
            FDT_END_NODE => {
                nodes.read_u32_be();
                break;
            }
            FDT_NOP => {
                nodes.read_u32_be();
                break;
            }
            FDT_PROP => {
                nodes.read_u32_be();
                let prop_len = nodes.read_u32_be();
                let name_off = nodes.read_u32_be();
                let mut name_off = name_off as usize;

                let name = read_null_term_str(strings, &mut name_off);

                let prop_val = nodes.read_bytes(prop_len as usize);
                nodes.align_to(4);

                properties.push((name, prop_val));
            }
            _ => panic!(),
        }
    }

    DeviceTreeNode {
        name,
        properties,
        childeren,
    }
}
