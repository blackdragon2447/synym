use core::ops::Range;
use core::{fmt::Write, slice};

use alloc::collections::btree_set::BTreeSet;

use crate::allocators::align_up;
use crate::{
    println,
    sync::{Mutex, SpinLock},
};

use super::boot_alloc::BOOT_HEAP;
use super::LinkedListAllocator;

mod sv48;
pub mod table;

pub struct Page {
    next: Option<*mut Page>,
}

static FREE_PAGES: Mutex<Option<*mut Page>, SpinLock> = Mutex::new(None);
static USED_PAGES: Mutex<
    Option<BTreeSet<*mut u8, &Mutex<LinkedListAllocator, SpinLock>>>,
    SpinLock,
> = Mutex::new(None);

pub fn init_page_alloc() {
    *USED_PAGES.lock().unwrap() = Some(BTreeSet::new_in(&*BOOT_HEAP));
}

pub fn add_pages_from_range(Range { start, end }: Range<usize>) {
    println!("Adding range: {:#X}..{:#X}", start as usize, end as usize);
    let start = align_up(start, 0x1000);

    let mut free = FREE_PAGES.lock().unwrap();

    let range = start..end;
    for page in range.clone().step_by(0x1000) {
        if !range.contains(&page) {
            panic!(
                "Page {:#X} outside range {:#X}..{:#X}",
                page, range.start, range.end
            );
        }
        // println!("{:#X}", page as usize);
        let next = core::mem::take(&mut *free);
        unsafe {
            *(page as *mut Page) = Page { next };
        }
        *free = Some(page as *mut Page);
    }
}

pub fn get_page() -> Option<*mut u8> {
    let mut free = FREE_PAGES.lock().unwrap();
    let mut used = USED_PAGES.lock().unwrap();

    if let Some(p) = *free {
        unsafe {
            *free = (*p).next;
        }

        used.as_mut()
            .expect("Page allocater need to be inited before its used")
            .insert(p as *mut u8);
        Some(p as *mut u8)
    } else {
        None
    }
}

pub fn get_zpage() -> Option<*mut u8> {
    get_page().map(|p| {
        unsafe {
            slice::from_raw_parts_mut(p as *mut u8, 0x1000)
                .iter_mut()
                .for_each(|m| *m = 0);
        }
        p
    })
}

pub fn return_page(page: *mut u8) {
    let mut used = USED_PAGES.lock().unwrap();

    if used
        .as_mut()
        .expect("Page allocater need to be inited before its used")
        .remove(&page)
    {
        let mut free = FREE_PAGES.lock().unwrap();

        let page = page as *mut Page;
        let next = core::mem::take(&mut *free);
        unsafe {
            *page = Page { next };
        }
        *free = Some(page);
    } else {
        panic!("Trying to return a page which was never taken")
    }
}

pub fn print_free_pages() {
    let mut list = *FREE_PAGES.lock().unwrap();

    while let Some(p) = list {
        unsafe {
            println!("{:#X}", p as usize);
            list = (*p).next;
        }
    }
}
