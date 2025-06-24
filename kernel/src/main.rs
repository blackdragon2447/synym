#![feature(
    naked_functions,
    str_from_raw_parts,
    debug_closure_helpers,
    allocator_api,
    iter_collect_into,
    unsafe_cell_access
)]
#![no_std]
#![no_main]
#![allow(dead_code)]

extern crate alloc;

use core::{arch::naked_asm, ops::Range, panic::PanicInfo};

use alloc::{format, vec::Vec};
use allocators::{
    align_up, init_global_alloc,
    paging::{
        add_pages_from_range, get_zpage,
        table::{PTEFlags, PTLevel, PageTable},
    },
};
use csr::Csr;
#[cfg(feature = "runtime-devicetree")]
use devtree::Dtb;
use enumflags2::make_bitflags;
use io::klog::{get_loglevel, set_loglevel, LogLevel};

mod allocators;
mod csr;
mod devtree;
mod io;
mod sbi;
mod sync;

extern "C" {
    static _text_start: usize;
    static _text_end: usize;

    static _rodata_start: usize;
    static _rodata_end: usize;

    static _data_start: usize;
    static _data_end: usize;

    static _bss_start: usize;
    static _bss_end: usize;

    static _stack_start: usize;
    static _stack_end: usize;

    static _heap_start: usize;
    static _heap_end: usize;
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    error!("{info}");
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

fn print_banner() {
    let text = "
 ____                              
/ ___| _   _ _ __  _   _ _ __ ___  
\\___ \\| | | | '_ \\| | | | '_ ` _ \\ 
 ___) | |_| | | | | |_| | | | | | |
|____/ \\__, |_| |_|\\__, |_| |_| |_|
       |___/       |___/           
\n";
    println!("{text}");
    println!("Hello World!\n");
}

#[no_mangle]
extern "C" fn kinit(
    _hartid: usize,
    #[cfg(feature = "runtime-devicetree")] dtb_addr: *const u8,
) -> ! {
    set_loglevel(LogLevel::Trace);

    print_banner();

    let heap_start = &raw const _heap_start as usize;
    debug!("heap_start: {:#X}", heap_start);
    let heap_end = &raw const _heap_end as usize;
    debug!("heap_end: {:#X}", heap_end);

    add_pages_from_range(heap_start..heap_end);

    init_global_alloc(get_zpage);

    debug!("Parsing devtree");

    #[cfg(feature = "hardcode-devicetree")]
    let devtree = devtree::decode_dtb(&devtree::DEVTREE);
    #[cfg(feature = "runtime-devicetree")]
    let dtb = unsafe { Dtb::from_pointer(dtb_addr) };
    #[cfg(feature = "runtime-devicetree")]
    let devtree = {
        let tree = devtree::decode_dtb(&dtb);
        tree
    };

    debug!("Memory nodes in devtree");
    let mut mem_regions = Vec::new();
    let mem_nodes = devtree.get_nodes("/memory");
    for mem in mem_nodes {
        assert!(mem.unit_name() == "memory");
        debug!("node: {}", mem.name());
        let reg = devtree.regs_for_node("/memory", mem.unit_addr()).unwrap();
        mem_regions.extend(reg.into_iter());
    }
    debug!("End memory nodes in devtree");

    if let Some(reseverd_mem) = devtree.get_nodes("/reserved-memory").pop() {
        for node in &reseverd_mem.childeren {
            debug!("{:#?}", node);
            let parent = "/reserved-memory/";
            let name = node.unit_name();
            let addr = node.unit_addr();
            let path = format!("{}{}", parent, name);

            let regs = devtree.regs_for_node(&path, addr);

            debug!("{:#?}", regs);
        }
    }

    debug!("Memory regions");
    for (a, s) in &mem_regions {
        debug!("reg_addr: {a:#X}, reg_size: {s:#X}, reg_end: {:#X}", a + s);
    }
    debug!("End memory regions");

    let heap_end = unsafe {
        let heap_end = &_heap_end as *const usize;
        let align = heap_end.align_offset(0x1000);
        let heap_end_aligned = heap_end.wrapping_add(align);
        debug!(
            "free pages start: {:#X} ({:#X})",
            heap_end as usize, heap_end_aligned as usize
        );
        heap_end_aligned as usize
    };

    info!("Collecting free pages");

    for (a, s) in &mem_regions {
        let range = *a..(a + s);
        if range.contains(&heap_end) {
            match sub_range(range, *a..heap_end) {
                (None, None) => {}
                (None, Some(r)) => add_pages_from_range(r),
                (Some(r), None) => add_pages_from_range(r),
                (Some(rl), Some(rr)) => {
                    add_pages_from_range(rl);
                    add_pages_from_range(rr);
                }
            }
        } else {
            add_pages_from_range(range);
        }
    }

    info!("Done collecting free pages");

    info!("Setting up paging for kernel");

    let pagetable = unsafe {
        let page = get_zpage().unwrap();
        let table = page as *mut PageTable;
        &mut *table
    };

    trace!("_text_start: {:#X}", &raw const _text_start as usize);
    trace!("_text_end: {:#X}", &raw const _text_end as usize);
    trace!(
        "_text_end (aligned): {:#X}",
        align_up(&raw const _text_end as usize, 0x1000),
    );

    for addr in ((&raw const _text_start as usize)
        ..(align_up(&raw const _text_end as usize, 0x1000)))
        .step_by(0x1000)
    {
        pagetable.map(
            addr,
            addr,
            make_bitflags!(PTEFlags::{Exec}),
            PTLevel::Page as u64,
        );
    }

    trace!("_rodata_start: {:#X}", &raw const _rodata_start as usize);
    trace!("_rodata_end: {:#X}", &raw const _rodata_end as usize);
    trace!(
        "_rodata_end (aligned): {:#X}",
        align_up(&raw const _rodata_end as usize, 0x1000),
    );

    for addr in ((&raw const _rodata_start as usize)
        ..(align_up(&raw const _rodata_end as usize, 0x1000)))
        .step_by(0x1000)
    {
        pagetable.map(
            addr,
            addr,
            make_bitflags!(PTEFlags::{Read}),
            PTLevel::Page as u64,
        );
    }

    trace!("_data_start: {:#X}", &raw const _data_start as usize);
    trace!("_data_end: {:#X}", &raw const _data_end as usize);
    trace!(
        "_data_end (aligned): {:#X}",
        align_up(&raw const _data_end as usize, 0x1000),
    );

    for addr in ((&raw const _data_start as usize)
        ..(align_up(&raw const _data_end as usize, 0x1000)))
        .step_by(0x1000)
    {
        pagetable.map(
            addr,
            addr,
            make_bitflags!(PTEFlags::{Read | Write}),
            PTLevel::Page as u64,
        );
    }

    trace!("_bss_start: {:#X}", &raw const _bss_start as usize);
    trace!("_bss_end: {:#X}", &raw const _bss_end as usize);
    trace!(
        "_bss_end (aligned): {:#X}",
        align_up(&raw const _bss_end as usize, 0x1000),
    );

    for addr in ((&raw const _bss_start as usize)..(align_up(&raw const _bss_end as usize, 0x1000)))
        .step_by(0x1000)
    {
        pagetable.map(
            addr,
            addr,
            make_bitflags!(PTEFlags::{Read | Write}),
            PTLevel::Page as u64,
        );
    }

    trace!("_stack_start: {:#X}", &raw const _stack_start as usize);
    trace!("_stack_end: {:#X}", &raw const _stack_end as usize);
    trace!(
        "_stack_end (aligned): {:#X}",
        align_up(&raw const _stack_end as usize, 0x1000),
    );

    for addr in ((&raw const _stack_start as usize)
        ..(align_up(&raw const _stack_end as usize, 0x1000)))
        .step_by(0x1000)
    {
        pagetable.map(
            addr,
            addr,
            make_bitflags!(PTEFlags::{Read | Write}),
            PTLevel::Page as u64,
        );
    }

    trace!("_heap_start: {:#X}", &raw const _heap_start as usize);
    trace!("_heap_end: {:#X}", &raw const _heap_end as usize);
    trace!(
        "_heap_end (aligned): {:#X}",
        align_up(&raw const _heap_end as usize, 0x1000),
    );

    for addr in ((&raw const _heap_start as usize)
        ..(align_up(&raw const _heap_end as usize, 0x1000)))
        .step_by(0x1000)
    {
        pagetable.map(
            addr,
            addr,
            make_bitflags!(PTEFlags::{Read | Write}),
            PTLevel::Page as u64,
        );
    }

    if get_loglevel() >= LogLevel::Debug {
        debug!("Set up paging for kernel, using the following page table");
        pagetable.print_recursive();
    } else {
        info!("Set up paging for kernel")
    }

    let satp = (9 << 60)
        | (0xffff << 44)
        | (((pagetable as *mut PageTable as u64) >> 12) & 0xfff_ffff_ffff);

    csr::Satp::write(satp);
    let satp_check = csr::Satp::read();
    assert_eq!(satp, satp_check, "Write to satp failed");
    info!("Successfully activated paging");

    #[allow(clippy::empty_loop)]
    loop {}
}

fn sub_range<T: PartialOrd>(lhs: Range<T>, rhs: Range<T>) -> (Option<Range<T>>, Option<Range<T>>) {
    if rhs.start <= lhs.start && rhs.end >= lhs.end {
        (None, None)
    } else if rhs.start <= lhs.start && rhs.end < lhs.end {
        (Some(rhs.end..lhs.end), None)
    } else if rhs.start > lhs.start && rhs.end >= lhs.end {
        (Some(rhs.start..lhs.end), None)
    } else if rhs.start > lhs.start && rhs.end < lhs.end {
        (Some(lhs.start..rhs.start), Some(rhs.end..lhs.end))
    } else {
        todo!()
    }
}
