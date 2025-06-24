use core::ops::{Index, IndexMut};

use enumflags2::{bitflags, BitFlags};

use crate::io::console::Indent;
use crate::{allocators::paging::get_zpage, println};

use super::sv48::{PAddr, PTEntry, VAddr};

#[repr(align(4096))]
#[derive(Debug)]
pub struct PageTable {
    entries: [PTEntry; 512],
}

#[bitflags]
#[repr(u64)]
#[derive(Clone, Copy, Debug)]
pub enum PTEFlags {
    Valid = 1 << 0,
    Read = 1 << 1,
    Write = 1 << 2,
    Exec = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Accessed = 1 << 6,
    Dirty = 1 << 7,
}

#[repr(u64)]
pub enum PTLevel {
    Page = 3,
    MegaPage = 2,
    GigaPage = 1,
    TerraPage = 0,
}

fn valid_rwx(flags: BitFlags<PTEFlags>) -> bool {
    if flags.contains(PTEFlags::Write) {
        flags.contains(PTEFlags::Read)
    } else {
        true
    }
}

impl PageTable {
    pub fn map<V: Into<VAddr>, P: Into<PAddr>>(
        &mut self,
        vaddr: V,
        paddr: P,
        flags: BitFlags<PTEFlags>,
        level: u64,
    ) {
        self.map_rec(vaddr, paddr, flags, level, 3);
    }

    pub fn map_rec<V: Into<VAddr>, P: Into<PAddr>>(
        &mut self,
        vaddr: V,
        paddr: P,
        flags: BitFlags<PTEFlags>,
        level: u64,
        vpn_nr: usize,
    ) {
        assert!(valid_rwx(flags));

        let vaddr: VAddr = vaddr.into();

        let vpn = vaddr.get_vpn();

        let entry = &mut self[vpn[vpn_nr]];

        if level > 0 {
            if !entry.is_valid() {
                let page = get_zpage().expect("No more free pages, we have ran out of memory");
                entry.set_ppn(PAddr(page as u64).get_ppn());
                entry.set_flags(PTEFlags::Valid.into());
            }
            let table = unsafe {
                let addr = PAddr::from_ppn(entry.get_ppn());
                &mut *(addr.0 as *mut PageTable)
            };
            table.map_rec(vaddr, paddr, flags, level - 1, vpn_nr - 1);
        } else {
            entry.set_ppn(paddr.into().get_ppn());
            entry.set_flags(flags | PTEFlags::Valid);
        }
    }

    pub fn print_recursive(&self) {
        println!("{:#X}:", self as *const PageTable as usize);
        self.print_recursive_int(0);
    }

    fn print_recursive_int(&self, indent: usize) {
        for entry in &self.entries {
            if entry.is_valid() {
                println!("{}{:?}", Indent(indent), entry);
                if !entry.is_leaf() {
                    unsafe { &*(PAddr::from_ppn(entry.get_ppn()).0 as *const PageTable) }
                        .print_recursive_int(indent + 1);
                }
            }
        }
    }
}

impl Index<usize> for PageTable {
    type Output = PTEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Index<u64> for PageTable {
    type Output = PTEntry;

    fn index(&self, index: u64) -> &Self::Output {
        &self[index as usize]
    }
}

impl IndexMut<u64> for PageTable {
    fn index_mut(&mut self, index: u64) -> &mut Self::Output {
        &mut self[index as usize]
    }
}
