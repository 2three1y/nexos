//! Kernel heap: 1 MiB of freshly mapped pages managed by a linked-list allocator.

use crate::memory::{self, HEAP_SIZE, HEAP_START};
use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::PageTableFlags;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init() -> Result<usize, &'static str> {
    let pages = memory::map_range(HEAP_START, HEAP_SIZE, PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE)?;
    unsafe { ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE as usize) };
    Ok(pages)
}

pub fn stats() -> (usize, usize) {
    let h = ALLOCATOR.lock();
    (h.used(), h.free())
}
