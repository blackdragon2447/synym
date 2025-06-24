// Based off https://os.phil-opp.com/allocator-designs/#linked-list-allocator
mod buddy_alloc;
pub mod paging;

pub use buddy_alloc::init_global_alloc;

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
pub fn align_up(addr: usize, align: usize) -> usize {
    assert_eq!(align.count_ones(), 1);
    (addr + align - 1) & !(align - 1)
}
