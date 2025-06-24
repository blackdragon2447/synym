use core::ops::Range;
use core::slice;

use crate::allocators::align_up;
use crate::sync::{Mutex, SpinLock};
use crate::trace;

mod sv48;
pub mod table;

struct Page {
    next: Option<*mut Page>,
}

pub const PAGE_SIZE: usize = 4096;

// enum UsedPages {
//     ConstVec(ConstVec<*mut u8, 512>),
//     BTreeSet(BTreeSet<*mut u8>),
// }

static FREE_PAGES: Mutex<Option<*mut Page>, SpinLock> = Mutex::new(None);
// static USED_PAGES: Mutex<UsedPages, SpinLock> = Mutex::new(UsedPages::ConstVec(ConstVec::new()));

// pub fn init_page_alloc() {
//     let used = USED_PAGES.lock().unwrap();
//     let UsedPages::ConstVec(ref vec) = *used else {
//         panic!("Double init of page alloc");
//     };
//     // We make a copy of the vec and
//     let vec = vec.clone();
//     used.unlock();
//
//     let mut set = BTreeSet::new();
//     for p in vec.iter() {
//         set.insert(*p);
//     }
//     let mut used = USED_PAGES.lock().unwrap();
//     *used = UsedPages::BTreeSet(set);
// }
//
// impl UsedPages {
//     fn insert(&mut self, ptr: *mut u8) {
//         match self {
//             UsedPages::ConstVec(const_vec) => {
//                 const_vec.push(ptr).unwrap();
//             }
//             UsedPages::BTreeSet(btree_set) => {
//                 btree_set.insert(ptr);
//             }
//         }
//     }
//
//     fn remove(&mut self, ptr: *mut u8) -> bool {
//         match self {
//             UsedPages::ConstVec(const_vec) => {
//                 let Some((idx, _)) = const_vec.iter().enumerate().find(|&(_, &p)| p == ptr) else {
//                     return false;
//                 };
//                 const_vec.swap_remove(idx);
//                 true
//             }
//             UsedPages::BTreeSet(btree_set) => btree_set.remove(&ptr),
//         }
//     }
// }

pub fn add_pages_from_range(Range { start, end }: Range<usize>) {
    trace!("Adding range: {:#X}..{:#X}", start, end);
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
    // let mut used = USED_PAGES.lock().unwrap();

    if let Some(p) = *free {
        unsafe {
            *free = (*p).next;
        }

        // used.insert(p as *mut u8);
        Some(p as *mut u8)
    } else {
        None
    }
}

pub fn get_zpage() -> Option<*mut u8> {
    get_page().inspect(|&p| unsafe {
        slice::from_raw_parts_mut(p, 0x1000)
            .iter_mut()
            .for_each(|m| *m = 0);
    })
}

pub fn return_page(page: *mut u8) {
    // let mut used = USED_PAGES.lock().unwrap();
    //
    // if used.remove(page) {
    let mut free = FREE_PAGES.lock().unwrap();

    let page = page as *mut Page;
    let next = core::mem::take(&mut *free);
    unsafe {
        *page = Page { next };
    }
    *free = Some(page);
    // } else {
    //     panic!("Trying to return a page which was never taken")
    // }
}
