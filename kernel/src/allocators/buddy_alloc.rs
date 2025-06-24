use core::{
    alloc::GlobalAlloc,
    fmt::Debug,
    mem::{self, size_of, MaybeUninit},
    ptr::{self, null_mut, NonNull},
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::{
    alloc::{AllocError, Layout},
    vec::Vec,
};
use static_assertions::const_assert;

use crate::{
    debug,
    sync::{Mutex, SpinLock},
};

use super::paging::PAGE_SIZE;

#[derive(Clone)]
enum BuddyBlock {
    Unallocated(BlockSize),
    Free(*mut u8, BlockSize),
    Used(*mut u8, BlockSize),
    Split(*mut BuddyBlock, *mut BuddyBlock, BlockSize),
}

impl Debug for BuddyBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BuddyBlock::Unallocated(block_size) => f
                .debug_tuple("BuddyBlock::Unallocated")
                .field(block_size)
                .finish(),
            BuddyBlock::Free(ptr, block_size) => f
                .debug_tuple("BuddyBlock::Free")
                .field(ptr)
                .field(block_size)
                .finish(),
            BuddyBlock::Used(ptr, block_size) => f
                .debug_tuple("BuddyBlock::Used")
                .field(ptr)
                .field(block_size)
                .finish(),
            BuddyBlock::Split(l, r, block_size) => f
                .debug_tuple("BuddyBlock::Split")
                .field(l)
                .field(unsafe { (*l).as_ref().unwrap() })
                .field(r)
                .field(unsafe { (*r).as_ref().unwrap() })
                .field(block_size)
                .finish(),
        }
    }
}

impl BuddyBlock {
    fn find_size_align(&mut self, size: BlockSize, align: usize) -> Option<&mut BuddyBlock> {
        match self {
            BuddyBlock::Unallocated(_) => None,
            BuddyBlock::Free(_, block_size) => {
                if size == *block_size && *block_size >= align {
                    Some(self)
                } else {
                    None
                }
            }
            BuddyBlock::Used(_, _) => None,
            BuddyBlock::Split(l, r, block_size) => {
                if *block_size > align {
                    if let Some(b) =
                        unsafe { (*l).as_mut().expect("This pointer should never be null") }
                            .find_size_align(size, align)
                    {
                        Some(b)
                    } else {
                        unsafe { (*r).as_mut().expect("This pointer should never be null") }
                            .find_size_align(size, align)
                    }
                } else if *block_size == align {
                    unsafe { (*l).as_mut().expect("This pointer should never be null") }
                        .find_size_align(size, align)
                } else {
                    None
                }
            }
        }
    }

    fn find_or_create_size_align<F>(
        &mut self,
        size: BlockSize,
        align: usize,
        alloc: &mut F,
    ) -> Option<&mut BuddyBlock>
    where
        F: FnMut() -> *mut BuddyBlock,
    {
        match self {
            BuddyBlock::Unallocated(_) => todo!(),
            BuddyBlock::Free(_, block_size) => {
                if size == *block_size && *block_size >= align {
                    Some(self)
                } else if size < *block_size && *block_size >= align {
                    let tmp = BuddyBlock::Split(null_mut(), null_mut(), *block_size);
                    let old = mem::replace(self, tmp);
                    let new = match old.split(&mut *alloc) {
                        Ok(n) => n,
                        Err(old) => {
                            *self = old;
                            return Some(self);
                        }
                    };

                    *self = new;

                    let BuddyBlock::Split(l, _, _) = self else {
                        panic!();
                    };

                    unsafe { l.as_mut().expect("This pointer should never be null") }
                        .find_or_create_size_align(size, align, alloc)
                } else {
                    None
                }
            }
            BuddyBlock::Used(_, _) => None,
            BuddyBlock::Split(l, r, block_size) => {
                if *block_size > align {
                    if let Some(b) =
                        unsafe { (*l).as_mut().expect("This pointer should never be null") }
                            .find_or_create_size_align(size, align, alloc)
                    {
                        Some(b)
                    } else {
                        unsafe { (*r).as_mut().expect("This pointer should never be null") }
                            .find_or_create_size_align(size, align, alloc)
                    }
                } else if *block_size == align {
                    unsafe { (*l).as_mut().expect("This pointer should never be null") }
                        .find_or_create_size_align(size, align, alloc)
                } else {
                    None
                }
            }
        }
    }

    fn free(&mut self, ptr: *mut u8) -> bool {
        match self {
            BuddyBlock::Unallocated(_) => false,
            BuddyBlock::Free(ptr_f, _) => {
                if ptr::eq(*ptr_f, ptr) {
                    panic!("Double free")
                } else {
                    false
                }
            }
            BuddyBlock::Used(ptr_u, block_size) => {
                if ptr::eq(*ptr_u, ptr) {
                    *self = BuddyBlock::Free(*ptr_u, *block_size);
                    true
                } else {
                    false
                }
            }
            BuddyBlock::Split(l, r, _) => {
                unsafe { (*r).as_mut().expect("This pointer should never be null") }.free(ptr)
                    || unsafe {
                        (*l).as_mut()
                            .expect("This pointer should never be null")
                            .free(ptr)
                    }
            }
        }
    }

    fn merge<F>(&mut self, reclaim_block: &mut F)
    where
        F: FnMut(*mut BuddyBlock),
    {
        match self {
            Self::Unallocated(_) => {}
            BuddyBlock::Free(_, _) => {}
            BuddyBlock::Used(_, _) => {}
            BuddyBlock::Split(l, r, block_size) => {
                if *block_size <= BlockSize::Bs4K {
                    unsafe { (*l).as_mut() }.unwrap().merge(&mut *reclaim_block);
                    unsafe { (*r).as_mut().unwrap() }.merge(&mut *reclaim_block);
                    if let (BuddyBlock::Free(ptr, _), BuddyBlock::Free(_, _)) =
                        unsafe { (l.as_ref().unwrap(), r.as_ref().unwrap()) }
                    {
                        reclaim_block(*l);
                        reclaim_block(*r);

                        *self = BuddyBlock::Free(*ptr, *block_size);
                    }
                } else {
                    unsafe { (*l).as_mut() }.unwrap().merge(&mut *reclaim_block);
                    unsafe { (*r).as_mut().unwrap() }.merge(&mut *reclaim_block);
                    if let (BuddyBlock::Unallocated(_), BuddyBlock::Unallocated(_)) =
                        unsafe { (l.as_ref().unwrap(), r.as_ref().unwrap()) }
                    {
                        reclaim_block(*l);
                        reclaim_block(*r);

                        *self = BuddyBlock::Unallocated(*block_size);
                    }
                }
            }
        }
    }

    fn split<F>(self, alloc: &mut F) -> Result<Self, Self>
    where
        F: FnMut() -> *mut BuddyBlock,
    {
        match self {
            BuddyBlock::Free(ptr, block_size) => {
                let Some(new_size) = block_size.split() else {
                    return Err(self);
                };
                let l = BuddyBlock::Free(ptr, new_size);
                let r = BuddyBlock::Free(unsafe { ptr.add(new_size as usize) }, new_size);

                let l_ptr = alloc();
                let r_ptr = alloc();

                unsafe {
                    l_ptr.write(l);
                    r_ptr.write(r);
                }

                Ok(BuddyBlock::Split(l_ptr, r_ptr, block_size))
            }
            BuddyBlock::Unallocated(block_size) => {
                let Some(new_size) = block_size.split() else {
                    return Err(self);
                };
                let l = BuddyBlock::Unallocated(new_size);
                let r = BuddyBlock::Unallocated(new_size);

                let l_ptr = alloc();
                let r_ptr = alloc();

                unsafe {
                    l_ptr.write(l);
                    r_ptr.write(r);
                }

                Ok(BuddyBlock::Split(l_ptr, r_ptr, block_size))
            }
            _ => panic!("Tried to split a non free block"),
        }
    }
}

#[repr(align(4096))]
struct BuddyBlockPage {
    blocks: [BuddyBlock; 128],
    used_blocks: u128,
    next: Option<*mut BuddyBlockPage>,
}

const_assert!(size_of::<BuddyBlockPage>() <= 0x1000);

impl BuddyBlockPage {
    fn get_block<F: FnMut() -> Option<*mut u8>>(&mut self, mut get_zpage: F) -> *mut BuddyBlock {
        if self.used_blocks != u128::MAX {
            for i in 0..128 {
                if (self.used_blocks >> i) & 1 == 0 {
                    self.used_blocks |= 1 << i;
                    return &raw mut self.blocks[i];
                }
            }

            unreachable!();
        } else if let Some(next) = self.next {
            unsafe { next.as_mut().unwrap().get_block(get_zpage) }
        } else {
            let page = get_zpage().unwrap() as *mut BuddyBlockPage;
            unsafe {
                page.as_mut().unwrap().next = None;
                page.as_mut().unwrap().used_blocks = 1;
            }
            self.next = Some(page);
            unsafe { &raw mut page.as_mut().unwrap().blocks[0] }
        }
    }

    fn return_block(&mut self, block: *mut BuddyBlock) {
        if (&raw mut *self as usize) == (block as usize & !0xfff) {
            let idx = unsafe { block.offset_from(self.blocks.as_ptr()) };
            self.used_blocks &= !(1 << idx);
        } else {
            let Some(page) = self.next.map(|p| unsafe { p.as_mut().unwrap() }) else {
                panic!("Tried to return a never taken out block")
            };
            page.return_block(block);
        }
    }
}

#[repr(usize)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum BlockSize {
    Bs1M = 0x100000,
    Bs512K = 0x80000,
    Bs256K = 0x40000,
    Bs128K = 0x20000,
    Bs64K = 0x10000,
    Bs32K = 0x8000,
    Bs16K = 0x4000,
    Bs8K = 0x2000,
    Bs4K = 0x1000,
    Bs2K = 0x800,
    Bs1K = 0x400,
    Bs512 = 0x200,
    Bs256 = 0x100,
    Bs128 = 0x80,
    Bs64 = 0x40,
    Bs32 = 0x20,
    Bs16 = 0x10,
    Bs8 = 0x8,
    Bs4 = 0x4,
}

impl BlockSize {
    fn split(self) -> Option<Self> {
        match self {
            BlockSize::Bs1M => Some(BlockSize::Bs512K),
            BlockSize::Bs512K => Some(BlockSize::Bs256K),
            BlockSize::Bs256K => Some(BlockSize::Bs128K),
            BlockSize::Bs128K => Some(BlockSize::Bs64K),
            BlockSize::Bs64K => Some(BlockSize::Bs32K),
            BlockSize::Bs32K => Some(BlockSize::Bs16K),
            BlockSize::Bs16K => Some(BlockSize::Bs8K),
            BlockSize::Bs8K => Some(BlockSize::Bs4K),
            BlockSize::Bs4K => Some(BlockSize::Bs2K),
            BlockSize::Bs2K => Some(BlockSize::Bs1K),
            BlockSize::Bs1K => Some(BlockSize::Bs512),
            BlockSize::Bs512 => Some(BlockSize::Bs256),
            BlockSize::Bs256 => Some(BlockSize::Bs128),
            BlockSize::Bs128 => Some(BlockSize::Bs64),
            BlockSize::Bs64 => Some(BlockSize::Bs32),
            BlockSize::Bs32 => Some(BlockSize::Bs16),
            BlockSize::Bs16 => Some(BlockSize::Bs8),
            BlockSize::Bs8 => Some(BlockSize::Bs4),
            BlockSize::Bs4 => None,
        }
    }
}

impl PartialEq<usize> for BlockSize {
    fn eq(&self, other: &usize) -> bool {
        (*self as usize).eq(other)
    }
}

impl PartialOrd<usize> for BlockSize {
    fn partial_cmp(&self, other: &usize) -> Option<core::cmp::Ordering> {
        (*self as usize).partial_cmp(other)
    }
}

impl TryFrom<usize> for BlockSize {
    type Error = usize;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value.next_power_of_two() {
            1 | 2 | 4 => Ok(Self::Bs4),
            8 => Ok(Self::Bs8),
            16 => Ok(Self::Bs16),
            32 => Ok(Self::Bs32),
            64 => Ok(Self::Bs64),
            128 => Ok(Self::Bs128),
            256 => Ok(Self::Bs256),
            512 => Ok(Self::Bs512),
            1024 => Ok(Self::Bs1K),
            2048 => Ok(Self::Bs2K),
            4096 => Ok(Self::Bs4K),
            n => Err(n),
        }
    }
}

struct BuddyAllocator {
    pages: *mut BuddyBlock,
    blocks: *mut BuddyBlockPage,
}

impl BuddyAllocator {
    pub fn new<F: FnMut() -> Option<*mut u8>>(mut get_zpage: F) -> Self {
        let mut blocks = BuddyBlockPage {
            blocks: [const { BuddyBlock::Unallocated(BlockSize::Bs1M) }; 128],
            used_blocks: 0,
            next: None,
        };
        Self {
            pages: {
                fn build_buddyblock<F: FnMut() -> Option<*mut u8>>(
                    size: BlockSize,
                    blocks: &mut BuddyBlockPage,
                    get_zpage: &mut F,
                ) -> *mut BuddyBlock {
                    if size > BlockSize::Bs4K {
                        let child_size = size.split().unwrap();
                        let left = build_buddyblock(child_size, blocks, get_zpage);
                        let right = build_buddyblock(child_size, blocks, get_zpage);

                        let block = BuddyBlock::Split(left, right, size);
                        let res = blocks.get_block(get_zpage);
                        unsafe {
                            res.write(block);
                        }
                        res
                    } else if size == BlockSize::Bs4K {
                        let block = BuddyBlock::Free(get_zpage().unwrap(), BlockSize::Bs4K);
                        let res = blocks.get_block(get_zpage);
                        unsafe {
                            res.write(block);
                        }
                        res
                    } else {
                        panic!("Don't call this with block_size less that a page")
                    }
                }
                build_buddyblock(BlockSize::Bs64K, &mut blocks, &mut get_zpage)
            },
            blocks: {
                let blocks_page = get_zpage().unwrap() as *mut BuddyBlockPage;
                unsafe { blocks_page.write(blocks) };
                blocks_page
            },
        }
    }

    fn get_block(
        new_blocks: &mut Vec<(usize, *mut BuddyBlock)>,
        reuse_blocks: &mut Vec<*mut BuddyBlock>,
    ) -> *mut BuddyBlock {
        if let Some(b) = reuse_blocks.pop() {
            return b;
        }

        for blocks in new_blocks {
            if blocks.0 + 1 < (PAGE_SIZE / size_of::<BuddyBlock>()) {
                let res = blocks.1;
                blocks.1 = blocks.1.wrapping_add(1);
                blocks.0 += 1;
                return res;
            }
        }

        panic!()
    }

    fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        match BlockSize::try_from(layout.size()) {
            Ok(size) => {
                let Self { pages, blocks } = self;
                let blocks = unsafe { blocks.as_mut().unwrap() };

                let pages = unsafe { pages.as_mut().unwrap() };

                if let Some(block) = pages.find_size_align(size, layout.align()) {
                    let BuddyBlock::Free(ptr, bs) = *block else {
                        panic!()
                    };

                    *block = BuddyBlock::Used(ptr, bs);

                    return Ok(NonNull::new(ptr).unwrap());
                }

                if let Some(block) =
                    pages.find_or_create_size_align(size, layout.align(), &mut || {
                        blocks.get_block(|| None)
                    })
                {
                    let BuddyBlock::Free(ptr, bs) = *block else {
                        panic!()
                    };

                    *block = BuddyBlock::Used(ptr, bs);

                    return Ok(NonNull::new(ptr).unwrap());
                }

                Err(AllocError)
            }
            Err(_) => todo!(),
        }
    }

    fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        let pages = unsafe { self.pages.as_mut().unwrap() };
        assert!(pages.free(ptr));
        pages.merge(&mut |p| unsafe {
            self.blocks.as_mut().unwrap().return_block(p);
        });
    }
}

pub fn init_global_alloc<F: FnMut() -> Option<*mut u8>>(get_zpage: F) {
    let (ref mut global_alloc, ref init) = *GLOBAL_ALLOC.0.lock().unwrap();

    let alloc = BuddyAllocator::new(get_zpage);

    global_alloc.write(alloc);
    init.store(true, Ordering::Release);

    debug!("Intialized global allocator");
}

struct GlobalBuddyAlloc(Mutex<(MaybeUninit<BuddyAllocator>, AtomicBool), SpinLock>);

unsafe impl GlobalAlloc for GlobalBuddyAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (ref mut alloc, ref init) = *self.0.lock().unwrap();

        if init.load(Ordering::Acquire) {
            alloc.assume_init_mut().alloc(layout).unwrap().as_ptr()
        } else {
            panic!("Please don't alloc with an uninitialised allocator");
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (ref mut alloc, ref init) = *self.0.lock().unwrap();

        if init.load(Ordering::Acquire) {
            alloc.assume_init_mut().dealloc(ptr, layout);
        } else {
            panic!("Please don't alloc with an uninitialised allocator");
        }
    }
}

#[global_allocator]
static GLOBAL_ALLOC: GlobalBuddyAlloc =
    GlobalBuddyAlloc(Mutex::new((MaybeUninit::uninit(), AtomicBool::new(false))));
