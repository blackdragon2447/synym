use core::fmt::Debug;

#[repr(C)]
pub struct Header {
    pub magic: u32,
    pub totalsize: u32,
    pub off_dt_struct: u32,
    pub off_dt_string: u32,
    pub off_mem_rsvmap: u32,
    pub version: u32,
    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,
    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}

impl Debug for Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Header")
            .field_with("magic", |f| write!(f, "{:#X}", self.magic))
            .field_with("totalsize", |f| write!(f, "{:#X}", self.totalsize))
            .field_with("off_dt_struct", |f| write!(f, "{:#X}", self.off_dt_struct))
            .field_with("off_dt_string", |f| write!(f, "{:#X}", self.off_dt_string))
            .field_with("off_mem_srvmap", |f| {
                write!(f, "{:#X}", self.off_mem_rsvmap)
            })
            .field_with("version", |f| write!(f, "{}", self.version))
            .field_with("last_comp_version", |f| {
                write!(f, "{}", self.last_comp_version)
            })
            .field_with("boot_cpuid_phys", |f| write!(f, "{}", self.boot_cpuid_phys))
            .field_with("size_dt_strings", |f| {
                write!(f, "{:#X}", self.size_dt_strings)
            })
            .field_with("size_dt_struct", |f| {
                write!(f, "{:#X}", self.size_dt_struct)
            })
            .finish()
    }
}

#[repr(C)]
pub struct FdtReserveEntry {
    pub addr: u64,
    pub size: u64,
}

impl Debug for FdtReserveEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FdtReserveEntry")
            .field_with("addr", |f| write!(f, "{:#X}", self.addr))
            .field_with("size", |f| write!(f, "{:#X}", self.size))
            .finish()
    }
}

pub struct FdtNodeProperty<'a> {
    name: &'a str,
    data: &'a [u8],
}

pub struct DebugLimList<'a, T: Debug>(pub &'a [T]);

impl<'a, T: Debug> Debug for DebugLimList<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0.len() <= 5 {
            f.debug_list().entries(self.0.iter()).finish()
        } else {
            f.debug_list()
                .entries(self.0.iter().take(5))
                .finish_non_exhaustive()
        }
    }
}
