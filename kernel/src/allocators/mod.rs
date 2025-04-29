// Based off https://os.phil-opp.com/allocator-designs/#linked-list-allocator
mod linked_list_alloc;

pub use linked_list_alloc::LinkedListAllocator;

use crate::sync::{Mutex, SpinLock};

// Note, we won't use this allocator for now, this is gonna be an allocator that uses paging,
// it is just here to make the alloc crate happy
#[global_allocator]
static GLOBAL_ALLOC: Mutex<LinkedListAllocator, SpinLock> = Mutex::new(LinkedListAllocator::new());

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
pub fn align_up(addr: usize, align: usize) -> usize {
    assert_eq!(align.count_ones(), 1);
    (addr + align - 1) & !(align - 1)
}
