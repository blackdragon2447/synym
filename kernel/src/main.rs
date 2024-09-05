#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod sbi;

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
unsafe extern "C" fn kernel_main(_arg0: usize) -> ! {
    let _ = sbi::debug_console::console_write("\nHello World!");
    loop {}
}
