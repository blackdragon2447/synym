#![feature(
    naked_functions,
    str_from_raw_parts,
    debug_closure_helpers,
    allocator_api,
    negative_impls,
    iter_collect_into
)]
#![no_std]
#![no_main]
#![allow(dead_code)]

extern crate alloc;

use core::{arch::naked_asm, fmt::Write, panic::PanicInfo, usize};

use alloc::vec::Vec;
use allocators::LinkedListAllocator;
use sbi::debug_console::Console;
use sync::{LazyLock, Mutex, SpinLock};

mod allocators;
mod bytesreader;
mod devtree;
mod sbi;
mod sync;

extern "C" {
    static _heap_start: usize;
    static _heap_end: usize;
    static _kernel_start: usize;
    static _kernel_end: usize;
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    writeln!(Console, "{info}");
    loop {}
}

#[naked]
#[link_section = ".text.init"]
#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    naked_asm!(
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
    )
}

fn print_hello() {
    let text = "
 ____                              
/ ___| _   _ _ __  _   _ _ __ ___  
\\___ \\| | | | '_ \\| | | | '_ ` _ \\ 
 ___) | |_| | | | | |_| | | | | | |
|____/ \\__, |_| |_|\\__, |_| |_| |_|
       |___/       |___/           
\n";
    let _ = sbi::debug_console::console_write(text);
    let _ = sbi::debug_console::console_write("\nHello World!\n");
}

#[no_mangle]
extern "C" fn kinit(_arg0: usize) -> ! {
    print_hello();

    let lock = Mutex::<&'static str, SpinLock>::new("");
    let mut guard = lock.lock().unwrap();
    *guard = "Hello Mutex\n";
    let lock = LazyLock::<&'static str, SpinLock>::new(|| "Hello LazyLock\n");

    let _ = sbi::debug_console::console_write(*guard);
    let _ = sbi::debug_console::console_write(*lock);
    let _ = sbi::debug_console::console_write("\n");

    let boot_heap = unsafe {
        let mut alloc = LinkedListAllocator::new();
        let heap_start = LinkedListAllocator::align_start(&_heap_start as *const usize as usize);
        let heap_size = &_heap_end as *const usize as usize - &_heap_start as *const usize as usize;
        writeln!(Console, "boot_heap_start: {:#X}", heap_start);
        writeln!(Console, "boot_heap_size: {:#X}\n", heap_size);
        alloc.init(heap_start, heap_size);
        Mutex::<LinkedListAllocator, SpinLock>::new(alloc)
    };

    let devtree = devtree::decode_dtb(&devtree::DEVTREE, &boot_heap);

    writeln!(Console, "Memory nodes in devtree");
    let mut mem_regions = Vec::new_in(&boot_heap);
    let mem_nodes = devtree.get_nodes("/memory", &boot_heap);
    for mem in mem_nodes {
        assert!(mem.unit_name() == "memory");
        writeln!(Console, "node: {}", mem.name());
        let reg = devtree
            .regs_for_node("/memory", mem.unit_addr(), &boot_heap)
            .unwrap();
        mem_regions.extend(reg.into_iter());
    }
    writeln!(Console, "End memory nodes in devtree\n");

    writeln!(Console, "Memory regions");
    for (a, s) in mem_regions {
        writeln!(Console, "reg_addr: {a:#X}, reg_size: {s:#X}");
    }
    writeln!(Console, "End memory regions\n");

    loop {}
}
