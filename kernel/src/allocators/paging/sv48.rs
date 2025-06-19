use core::{
    fmt::{Debug, Write},
    ops,
};

use enumflags2::{make_bitflags, BitFlags};

use super::table::PTEFlags;

#[repr(transparent)]
pub struct PTEntry(u64);

impl Debug for PTEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#X}; ", PAddr::from_ppn(self.get_ppn()).0)?;

        write!(f, "{:#X}; ", self.get_rsw())?;
        write!(f, "{}", self.get_flags())
    }
}

#[repr(transparent)]
pub struct VAddr(pub u64);

#[repr(transparent)]
pub struct PAddr(pub u64);

impl PTEntry {
    pub fn get_flags(&self) -> BitFlags<PTEFlags> {
        BitFlags::from_bits_truncate(self.0)
    }

    pub fn set_flags(&mut self, flags: BitFlags<PTEFlags>) {
        self.0 = (self.0 & !0xff) | flags.bits();
    }

    pub fn get_rsw(&self) -> u64 {
        (self.0 >> 8) & 0b11
    }

    pub fn set_rsw(&mut self, rsw: u64) {
        self.0 = (self.0 & !(0b11 << 8)) | ((rsw & 0b11) << 8);
    }

    pub fn get_ppn(&self) -> [u64; 4] {
        [
            (self.0 >> 10) & 0x1ff,
            (self.0 >> 19) & 0x1ff,
            (self.0 >> 28) & 0x1ff,
            (self.0 >> 37) & 0x1_ffff,
        ]
    }

    pub fn set_ppn(&mut self, ppn: [u64; 4]) {
        self.0 = (self.0 & !(0xfff_ffff_ffff << 10))
            | (ppn[0] & 0x1ff) << 10
            | (ppn[1] & 0x1ff) << 19
            | (ppn[2] & 0x1ff) << 28
            | (ppn[3] & 0x1_ffff) << 27;
    }

    pub fn is_valid(&self) -> bool {
        self.get_flags().contains(PTEFlags::Valid)
    }

    pub fn is_leaf(&self) -> bool {
        self.get_flags()
            .intersects(make_bitflags!(PTEFlags::{Read |  Write | Exec}))
    }
}

impl ops::BitOr<BitFlags<PTEFlags>> for PTEntry {
    type Output = PTEntry;

    fn bitor(self, rhs: BitFlags<PTEFlags>) -> Self::Output {
        Self(self.0 | rhs.bits())
    }
}

impl ops::BitAnd<BitFlags<PTEFlags>> for PTEntry {
    type Output = PTEntry;

    fn bitand(self, rhs: BitFlags<PTEFlags>) -> Self::Output {
        Self(self.0 & (!0xff | rhs.bits()))
    }
}

impl VAddr {
    pub fn get_vpn(&self) -> [u64; 4] {
        [
            (self.0 >> 12) & 0x1ff,
            (self.0 >> 21) & 0x1ff,
            (self.0 >> 30) & 0x1ff,
            (self.0 >> 39) & 0x1ff,
        ]
    }
}

impl From<u64> for VAddr {
    fn from(value: u64) -> Self {
        Self(value & 0x7fff_ffff_ffff)
    }
}

impl PAddr {
    pub fn get_ppn(&self) -> [u64; 4] {
        [
            (self.0 >> 12) & 0x1ff,
            (self.0 >> 21) & 0x1ff,
            (self.0 >> 30) & 0x1ff,
            (self.0 >> 39) & 0x1_ffff,
        ]
    }

    pub fn from_ppn(ppn: [u64; 4]) -> Self {
        Self(
            ((ppn[0] & 0x1ff) << 12)
                | ((ppn[1] & 0x1ff) << 21)
                | ((ppn[2] & 0x1ff) << 30)
                | ((ppn[3] & 0x1_ffff) << 39),
        )
    }
}

impl From<u64> for PAddr {
    fn from(value: u64) -> Self {
        Self(value & 0x7f_ffff_ffff_ffff)
    }
}
