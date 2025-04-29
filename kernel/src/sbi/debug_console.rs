use core::fmt;

use super::{SbiArgs, SbiResult};

pub struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        console_write(s).map(|_| ()).map_err(|_| fmt::Error)
    }
}

pub fn console_write(string: &str) -> SbiResult {
    let args = SbiArgs {
        a0: string.len(),
        a1: string.as_ptr() as usize,
        ..Default::default()
    };
    let ret = unsafe { super::sbi_call(0x4442434E, 0x0, args) };
    ret.into_result()
}
