pub mod debug_console;

use core::arch::asm;

use num_enum::FromPrimitive;

#[repr(isize)]
#[derive(FromPrimitive, Clone, Copy)]
#[allow(dead_code)]
pub enum SbiError {
    Success = 0isize,
    Failed = -1isize,
    NotSupported = -2isize,
    InvalidParam = -3isize,
    Denied = -4isize,
    InvalidAddress = -5isize,
    AlreadyAvailable = -6isize,
    AlreadyStarted = -7isize,
    AlreadyStopped = -8isize,
    SharedMemoryNotAvailable = -9isize,
    #[num_enum(catch_all)]
    Unknown(isize) = -10,
}

#[derive(Default)]
struct SbiRet {
    error: isize,
    value: usize,
}

pub type SbiResult = Result<usize, SbiError>;

impl SbiRet {
    fn into_result(self) -> SbiResult {
        let err = SbiError::from(self.error);

        match err {
            SbiError::Success => Ok(self.value),
            e => Err(e),
        }
    }
}

#[derive(Default)]
struct SbiArgs {
    a0: usize,
    a1: usize,
    a2: usize,
}

unsafe fn sbi_call(eid: usize, fid: usize, SbiArgs { a0, a1, a2 }: SbiArgs) -> SbiRet {
    let mut ret: SbiRet = Default::default();
    asm!(
        "ecall",
        in("a7") eid,
        in("a6") fid,
        inout("a0") a0 => ret.error,
        inout("a1") a1 => ret.value,
        in("a2") a2,
    );

    ret
}
