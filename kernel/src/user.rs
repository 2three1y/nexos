//! Loading and running ring-3 programs.
//!
//! The program registry: every app's `entry` name (from its manifest in
//! kernel/catalog/) maps to its code here, either a ring-3 ELF built from
//! userland/ and embedded in the kernel image, or a native kernel app.
//! What is installed, and everything users see about an app, comes from the
//! App Store's manifests (apps/store.rs); this table only holds the code.

use crate::memory::{self, USER_BASE, USER_LIMIT};
use crate::{gdt, syscall};
use x86_64::structures::paging::PageTableFlags;

static HELLO_ELF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/userland.elf"));
static GUESS_ELF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guess.elf"));

pub enum AppKind {
    /// A ring-3 ELF program (built from userland/).
    Elf(fn() -> &'static [u8]),
    /// A built-in full-screen app that runs inside the kernel (kernel/src/apps/).
    Native(fn()),
}

pub struct App {
    /// The `entry` name a manifest points at.
    pub name: &'static str,
    pub kind: AppKind,
}

pub static APPS: &[App] = &[
    App { name: "hello", kind: AppKind::Elf(|| HELLO_ELF) },
    App { name: "guess", kind: AppKind::Elf(|| GUESS_ELF) },
    App { name: "store", kind: AppKind::Native(crate::apps::store::run) },
    App { name: "notes", kind: AppKind::Native(crate::apps::notes::run) },
    App { name: "calc", kind: AppKind::Native(crate::apps::calc::run) },
    App { name: "clock", kind: AppKind::Native(crate::apps::clock::run) },
    App { name: "sysinfo", kind: AppKind::Native(crate::apps::sysinfo::run) },
    App { name: "piano", kind: AppKind::Native(crate::apps::piano::run) },
    App { name: "insomnia", kind: AppKind::Native(crate::apps::insomnia::run) },
];

/// Size of an app's program image in bytes, for ring-3 apps.
pub fn image_size(name: &str) -> Option<usize> {
    match find(name)?.kind {
        AppKind::Elf(image) => Some(image().len()),
        AppKind::Native(_) => None,
    }
}

const USER_STACK_TOP: u64 = USER_BASE + 0x80_0000;
const USER_STACK_SIZE: u64 = 16 * 1024;

fn rd16(b: &[u8], o: usize) -> u64 { u16::from_le_bytes([b[o], b[o + 1]]) as u64 }
fn rd32(b: &[u8], o: usize) -> u64 { u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as u64 }
fn rd64(b: &[u8], o: usize) -> u64 { u64::from_le_bytes(b[o..o + 8].try_into().unwrap()) }

/// Load an ELF64 executable into the user window; returns its entry point.
fn load_elf(elf: &[u8]) -> Result<u64, &'static str> {
    if elf.len() < 64 || &elf[0..4] != b"\x7fELF" || elf[4] != 2 {
        return Err("not an ELF64 image (build userland first: make)");
    }
    let entry = rd64(elf, 24);
    let phoff = rd64(elf, 32) as usize;
    let phentsize = rd16(elf, 54) as usize;
    let phnum = rd16(elf, 56) as usize;
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        if rd32(elf, ph) != 1 {
            continue; // not PT_LOAD
        }
        let offset = rd64(elf, ph + 8) as usize;
        let vaddr = rd64(elf, ph + 16);
        let filesz = rd64(elf, ph + 32) as usize;
        let memsz = rd64(elf, ph + 40);
        if memsz == 0 {
            continue;
        }
        if vaddr < USER_BASE || vaddr + memsz > USER_LIMIT || offset + filesz > elf.len() {
            return Err("segment outside the user window");
        }
        memory::map_range(vaddr, memsz, PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE)?;
        unsafe {
            let dst = vaddr as *mut u8;
            core::ptr::write_bytes(dst, 0, memsz as usize);
            core::ptr::copy_nonoverlapping(elf[offset..].as_ptr(), dst, filesz);
        }
    }
    Ok(entry)
}

pub fn find(name: &str) -> Option<&'static App> {
    APPS.iter().find(|a| a.name == name)
}

/// Run an app in ring 3 and return its exit code.
pub fn run(app: &App) -> Result<i64, &'static str> {
    let image = match app.kind {
        AppKind::Native(f) => {
            f();
            return Ok(0);
        }
        AppKind::Elf(image) => image(),
    };
    let entry = load_elf(image)?;
    memory::map_range(
        USER_STACK_TOP - USER_STACK_SIZE,
        USER_STACK_SIZE,
        PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
    )?;
    let sel = gdt::selectors();
    let code = unsafe {
        syscall::enter_user(entry, USER_STACK_TOP, sel.user_code.0 as u64, sel.user_data.0 as u64)
    };
    x86_64::instructions::interrupts::enable();
    Ok(code)
}
