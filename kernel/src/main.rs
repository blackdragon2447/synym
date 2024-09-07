#![feature(naked_functions)]
#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo};

mod sbi;

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}

#[naked]
#[link_section = ".text.init"]
#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    asm!(
        ".option push",
        ".option norelax",
        "   la      gp, __global_pointer$",
        "   la      t3, _bss_start",
        "   la      t4, _bss_end",
        ".option pop",
        "1:",
        "   bge     t3, t4, 1f",
        "   sd      zero, 0(t3)",
        "   addi    t3, t3, 8",
        "   j       1b",
        "1:",
        "   la      sp, _stack_end",
        "   li      t0, (1 << 8) | (1 << 5) | (1 << 13)",
        "   csrw    sstatus, t0",
        "   la      t1, kinit",
        "   csrw    sie, zero",
        "   csrw    sepc, t1",
        "   sret",
        options(noreturn)
    )
}

#[no_mangle]
unsafe extern "C" fn kinit(_arg0: usize) -> ! {
    let _ = sbi::debug_console::console_write("\nHello World!");
    loop {}
}
