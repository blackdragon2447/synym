use super::{SbiArgs, SbiResult};

pub fn console_write(string: &str) -> SbiResult {
    let args = SbiArgs {
        a0: string.len(),
        a1: string.as_ptr() as usize,
        ..Default::default()
    };
    let ret = unsafe { super::sbi_call(0x4442434E, 0x0, args) };
    ret.into_result()
}
