//! Physical memory (Multiboot2 memory map + frame allocator) and paging,
//! i686 build: classic 32-bit two-level paging (no PAE, so it runs on any
//! i686-class CPU).
//!
//! The boot stub identity-maps the first 1 GiB with 4 MiB pages. On top of
//! that, the kernel maps fresh 4 KiB pages through real page tables for the
//! heap (at HEAP_START) and for ring-3 programs (USER_BASE..).

use core::arch::asm;
use spin::Mutex;

pub const HEAP_START: u64 = 0x5000_0000;
pub const HEAP_SIZE: u64 = 1024 * 1024;
pub const USER_BASE: u64 = 0x4000_0000;
pub const USER_LIMIT: u64 = 0x4100_0000;

/// Page flags (the low bits of a 32-bit page-table entry).
#[derive(Clone, Copy)]
pub struct Flags(u32);
const PRESENT: u32 = 1;
const WRITABLE: u32 = 2;
const USER: u32 = 4;
const HUGE: u32 = 0x80;

/// Kernel heap: writable, kernel only. (No NX bit without PAE.)
pub const HEAP_FLAGS: Flags = Flags(WRITABLE);
/// Ring-3 program image: user accessible and writable.
pub const USER_FLAGS: Flags = Flags(USER | WRITABLE);
/// Ring-3 stack.
pub const USER_STACK_FLAGS: Flags = Flags(USER | WRITABLE);

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

unsafe fn rd32(a: u64) -> u32 {
    core::ptr::read_unaligned(a as usize as *const u32)
}
unsafe fn rd64(a: u64) -> u64 {
    core::ptr::read_unaligned(a as usize as *const u64)
}

/// Parse the Multiboot2 boot information structure.
pub unsafe fn parse_multiboot(addr: u64) -> BootInfo {
    let mut info = BootInfo { loader: [0; 48], loader_len: 0, regions: [(0, 0); MAX_REGIONS], region_count: 0, info_end: 0 };
    let total = rd32(addr) as u64;
    info.info_end = addr + total;
    let mut tag = addr + 8;
    while tag < addr + total {
        let ty = rd32(tag);
        let size = rd32(tag + 4) as u64;
        match ty {
            0 => break,
            2 => {
                let s = (tag + 8) as usize as *const u8;
                let mut n = 0;
                while n < 48 && *s.add(n) != 0 {
                    info.loader[n] = *s.add(n);
                    n += 1;
                }
                info.loader_len = n;
            }
            6 => {
                let entry_size = rd32(tag + 8) as u64;
                let mut e = tag + 16;
                while e + entry_size <= tag + size {
                    let base = rd64(e);
                    let len = rd64(e + 8);
                    let kind = rd32(e + 16);
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
        let kend = unsafe { &__kernel_end as *const u8 as usize as u64 };
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

    fn allocate(&mut self) -> Option<u32> {
        while self.region < self.count {
            let (_, end) = self.regions[self.region];
            if self.next + 4096 <= end {
                let f = self.next as u32;
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
    pd: u32,
}

pub static MEMORY: Mutex<Option<Memory>> = Mutex::new(None);

pub fn init(info: &BootInfo) {
    let cr3: usize;
    unsafe { asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack)) };
    *MEMORY.lock() = Some(Memory { frames: BumpFrameAllocator::new(info), pd: (cr3 & !0xFFF) as u32 });
}

fn table(phys: u32) -> &'static mut [u32; 1024] {
    // Page tables live below 1 GiB, which is identity mapped.
    unsafe { &mut *(phys as usize as *mut [u32; 1024]) }
}

/// Map `[start, start+len)` to fresh frames with the given flags.
/// Pages that are already mapped are left as they are.
pub fn map_range(start: u64, len: u64, flags: Flags) -> Result<usize, &'static str> {
    let mut guard = MEMORY.lock();
    let mem = guard.as_mut().ok_or("memory not initialised")?;
    if start + len > 0x1_0000_0000 {
        return Err("address beyond 4 GiB");
    }
    let pd = table(mem.pd);
    let mut mapped = 0;
    let mut va = (start as u32) & !0xFFF;
    let end = (start + len - 1) as u32;
    loop {
        let pdi = (va >> 22) as usize;
        let pde = pd[pdi];
        if pde & PRESENT != 0 && pde & HUGE != 0 {
            // inside the identity-mapped 4 MiB pages: already mapped
        } else {
            if pde & PRESENT == 0 {
                let pt = mem.frames.allocate().ok_or("out of physical frames")?;
                table(pt).fill(0);
                // The directory entry allows user + write; the page entry decides.
                pd[pdi] = pt | PRESENT | WRITABLE | USER;
            }
            let pt = table(pd[pdi] & !0xFFF);
            let pti = ((va >> 12) & 0x3FF) as usize;
            if pt[pti] & PRESENT == 0 {
                let frame = mem.frames.allocate().ok_or("out of physical frames")?;
                pt[pti] = frame | flags.0 | PRESENT;
                unsafe { asm!("invlpg [{}]", in(reg) va as usize, options(nostack)) };
                mapped += 1;
            }
        }
        match va.checked_add(4096) {
            Some(n) if n <= end => va = n,
            _ => break,
        }
    }
    Ok(mapped)
}

pub fn translate(addr: u64) -> Option<u64> {
    let guard = MEMORY.lock();
    let mem = guard.as_ref()?;
    let va = addr as u32;
    let pde = table(mem.pd)[(va >> 22) as usize];
    if pde & PRESENT == 0 {
        return None;
    }
    if pde & HUGE != 0 {
        return Some(((pde & 0xFFC0_0000) | (va & 0x3F_FFFF)) as u64);
    }
    let pte = table(pde & !0xFFF)[((va >> 12) & 0x3FF) as usize];
    if pte & PRESENT == 0 {
        return None;
    }
    Some(((pte & !0xFFF) | (va & 0xFFF)) as u64)
}

pub fn frame_stats() -> (u64, u64) {
    let guard = MEMORY.lock();
    guard.as_ref().map(|m| (m.frames.used, m.frames.total)).unwrap_or((0, 0))
}
