use crate::{
    _heap_end, _heap_start, println,
    sync::{LazyLock, Mutex, SpinLock},
};
use core::fmt::Write;

use super::LinkedListAllocator;

pub static BOOT_HEAP: LazyLock<Mutex<LinkedListAllocator, SpinLock>, SpinLock> =
    LazyLock::new(|| unsafe {
        let mut alloc = LinkedListAllocator::new();
        let heap_start = LinkedListAllocator::align_start(&_heap_start as *const usize as usize);
        let heap_size = &_heap_end as *const usize as usize - &_heap_start as *const usize as usize;
        println!("boot_heap_start: {:#X}", heap_start);
        println!("boot_heap_size: {:#X}\n", heap_size);
        alloc.init(heap_start, heap_size);
        Mutex::<LinkedListAllocator, SpinLock>::new(alloc)
    });
