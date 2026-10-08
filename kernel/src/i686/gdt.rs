//! Global Descriptor Table + Task State Segment (i686 build).
//!
//! Flat 4 GiB kernel and user (ring 3) segments in the same order as the
//! x86_64 build (kernel code, kernel data, user data, user code, TSS), plus a
//! 32-bit TSS whose ESP0/SS0 give the stack the CPU switches to when an
//! interrupt or a system call arrives while a ring-3 program is running.

use core::arch::asm;

const STACK_SIZE: usize = 4096 * 5;

#[repr(align(16))]
#[allow(dead_code)]
struct Stack([u8; STACK_SIZE]);
static mut ESP0_STACK: Stack = Stack([0; STACK_SIZE]);

#[derive(Clone, Copy)]
pub struct Sel(pub u16);

#[allow(dead_code)]
pub struct Selectors {
    pub kernel_code: Sel,
    pub kernel_data: Sel,
    pub user_code: Sel,
    pub user_data: Sel,
}

static SELECTORS: Selectors = Selectors {
    kernel_code: Sel(0x08),
    kernel_data: Sel(0x10),
    user_data: Sel(0x18 | 3),
    user_code: Sel(0x20 | 3),
};

#[repr(C, packed)]
struct Tss {
    link: u32,
    esp0: u32,
    ss0: u32,
    rest: [u32; 22], // esp1..ldt: unused
    trap: u16,
    iomap: u16,
}

static mut TSS: Tss = Tss { link: 0, esp0: 0, ss0: 0x10, rest: [0; 22], trap: 0, iomap: 104 };

static mut GDT: [u64; 6] = [0; 6];

const fn seg(base: u32, limit: u32, access: u8, flags: u8) -> u64 {
    (limit as u64 & 0xFFFF)
        | ((base as u64 & 0xFF_FFFF) << 16)
        | ((access as u64) << 40)
        | (((limit as u64 >> 16) & 0xF) << 48)
        | (((flags as u64) & 0xF) << 52)
        | (((base as u64 >> 24) & 0xFF) << 56)
}

#[repr(C, packed)]
struct Pointer {
    limit: u16,
    base: u32,
}

pub fn init() {
    unsafe {
        TSS.esp0 = (&raw const ESP0_STACK as u32) + STACK_SIZE as u32;
        let tss_base = &raw const TSS as u32;
        GDT[1] = seg(0, 0xFFFFF, 0x9A, 0xC); // kernel code: present, ring 0, exec/read, 4K gran, 32-bit
        GDT[2] = seg(0, 0xFFFFF, 0x92, 0xC); // kernel data
        GDT[3] = seg(0, 0xFFFFF, 0xF2, 0xC); // user data (ring 3)
        GDT[4] = seg(0, 0xFFFFF, 0xFA, 0xC); // user code (ring 3)
        GDT[5] = seg(tss_base, core::mem::size_of::<Tss>() as u32 - 1, 0x89, 0x0); // 32-bit TSS
        let ptr = Pointer { limit: (core::mem::size_of::<[u64; 6]>() - 1) as u16, base: &raw const GDT as u32 };
        asm!(
            "lgdt [{p}]",
            "push 0x08",
            "lea {t}, [2f]",
            "push {t}",
            "retf",
            "2:",
            "mov {t:x}, 0x10",
            "mov ds, {t:x}",
            "mov es, {t:x}",
            "mov fs, {t:x}",
            "mov gs, {t:x}",
            "mov ss, {t:x}",
            "mov {t:x}, 0x28",
            "ltr {t:x}",
            p = in(reg) &ptr,
            t = out(reg) _,
        );
    }
}

pub fn selectors() -> &'static Selectors {
    &SELECTORS
}
