//! Physical memory (Multiboot2 memory map + frame allocator) and paging.
//!
//! The boot stub identity-maps the first 1 GiB with 2 MiB pages. On top of
//! that, the kernel maps fresh 4 KiB pages through the real page tables for
//! the heap (at HEAP_START) and for ring-3 programs (USER_BASE..).

use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB, Translate,
};
use x86_64::{PhysAddr, VirtAddr};

pub const HEAP_START: u64 = 0x4444_4444_0000;
pub const HEAP_SIZE: u64 = 1024 * 1024;
pub const USER_BASE: u64 = 0x4000_0000;
pub const USER_LIMIT: u64 = 0x4100_0000;

extern "C" {
    static __kernel_end: u8;
}

const MAX_REGIONS: usize = 32;

pub struct BootInfo {
    pub loader: [u8; 48],
    pub loader_len: usize,
    regions: [(u64, u64); MAX_REGIONS],
    region_count: usize,
    pub info_end: u64,
}

impl BootInfo {
    pub fn loader_name(&self) -> &str {
        core::str::from_utf8(&self.loader[..self.loader_len]).unwrap_or("?")
    }
    pub fn usable_bytes(&self) -> u64 {
        self.regions[..self.region_count].iter().map(|r| r.1).sum()
    }
}

/// Parse the Multiboot2 boot information structure.
pub unsafe fn parse_multiboot(addr: u64) -> BootInfo {
    let mut info = BootInfo { loader: [0; 48], loader_len: 0, regions: [(0, 0); MAX_REGIONS], region_count: 0, info_end: 0 };
    let total = *(addr as *const u32) as u64;
    info.info_end = addr + total;
    let mut tag = addr + 8;
    while tag < addr + total {
        let ty = *(tag as *const u32);
        let size = *((tag + 4) as *const u32) as u64;
        match ty {
            0 => break,
            2 => {
                // boot loader name (NUL-terminated)
                let s = (tag + 8) as *const u8;
                let mut n = 0;
                while n < 48 && *s.add(n) != 0 {
                    info.loader[n] = *s.add(n);
                    n += 1;
                }
                info.loader_len = n;
            }
            6 => {
                // memory map
                let entry_size = *((tag + 8) as *const u32) as u64;
                let mut e = tag + 16;
                while e + entry_size <= tag + size {
                    let base = *(e as *const u64);
                    let len = *((e + 8) as *const u64);
                    let kind = *((e + 16) as *const u32);
                    if kind == 1 && info.region_count < MAX_REGIONS {
                        info.regions[info.region_count] = (base, len);
                        info.region_count += 1;
                    }
                    e += entry_size;
                }
            }
            _ => {}
        }
        tag += (size + 7) & !7;
    }
    info
}

/// Hands out 4 KiB physical frames from the usable regions, above the kernel
/// image and the boot info, below 1 GiB (so they stay identity mapped).
pub struct BumpFrameAllocator {
    regions: [(u64, u64); MAX_REGIONS],
    count: usize,
    region: usize,
    next: u64,
    pub used: u64,
    pub total: u64,
}

impl BumpFrameAllocator {
    pub fn new(info: &BootInfo) -> Self {
        let kend = unsafe { &__kernel_end as *const u8 as u64 };
        let floor = (kend.max(info.info_end) + 0xFFF) & !0xFFF;
        let mut regions = [(0u64, 0u64); MAX_REGIONS];
        let mut count = 0;
        let mut total = 0;
        for &(base, len) in &info.regions[..info.region_count] {
            let start = ((base.max(floor)) + 0xFFF) & !0xFFF;
            let end = (base + len).min(0x4000_0000) & !0xFFF;
            if end > start {
                regions[count] = (start, end);
                total += (end - start) / 4096;
                count += 1;
            }
        }
        let next = if count > 0 { regions[0].0 } else { 0 };
        BumpFrameAllocator { regions, count, region: 0, next, used: 0, total }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BumpFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        while self.region < self.count {
            let (_, end) = self.regions[self.region];
            if self.next + 4096 <= end {
                let f = PhysFrame::containing_address(PhysAddr::new(self.next));
                self.next += 4096;
                self.used += 1;
                return Some(f);
            }
            self.region += 1;
            if self.region < self.count {
                self.next = self.regions[self.region].0;
            }
        }
        None
    }
}

pub struct Memory {
    pub frames: BumpFrameAllocator,
    pub mapper: OffsetPageTable<'static>,
}

pub static MEMORY: Mutex<Option<Memory>> = Mutex::new(None);

pub fn init(info: &BootInfo) {
    let (p4_frame, _) = Cr3::read();
    // Physical memory is identity mapped, so the physical-memory offset is 0.
    let p4 = unsafe { &mut *(p4_frame.start_address().as_u64() as *mut PageTable) };
    let mapper = unsafe { OffsetPageTable::new(p4, VirtAddr::new(0)) };
    *MEMORY.lock() = Some(Memory { frames: BumpFrameAllocator::new(info), mapper });
}

/// Map `[start, start+len)` to fresh frames with the given flags.
/// Pages that are already mapped are left as they are.
pub fn map_range(start: u64, len: u64, flags: PageTableFlags) -> Result<usize, &'static str> {
    let mut guard = MEMORY.lock();
    let mem = guard.as_mut().ok_or("memory not initialised")?;
    let first = Page::<Size4KiB>::containing_address(VirtAddr::new(start));
    let last = Page::<Size4KiB>::containing_address(VirtAddr::new(start + len - 1));
    let mut mapped = 0;
    for page in Page::range_inclusive(first, last) {
        if mem.mapper.translate_page(page).is_ok() {
            continue;
        }
        let frame = mem.frames.allocate_frame().ok_or("out of physical frames")?;
        unsafe {
            mem.mapper
                .map_to(page, frame, flags | PageTableFlags::PRESENT, &mut mem.frames)
                .map_err(|_| "map_to failed")?
                .flush();
        }
        mapped += 1;
    }
    Ok(mapped)
}

pub fn translate(addr: u64) -> Option<u64> {
    let guard = MEMORY.lock();
    guard.as_ref()?.mapper.translate_addr(VirtAddr::new(addr)).map(|p| p.as_u64())
}

pub fn frame_stats() -> (u64, u64) {
    let guard = MEMORY.lock();
    guard.as_ref().map(|m| (m.frames.used, m.frames.total)).unwrap_or((0, 0))
}
