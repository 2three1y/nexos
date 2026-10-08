//! NexOS — the kernel.
//!
//! Boot path: GRUB (Multiboot2) -> boot.rs (32-bit stub; long mode on x86_64,
//! plain protected mode with 2-level paging on i686 from i686/boot.rs) ->
//! `kernel_main`, which brings up serial + VGA, GDT/TSS, IDT, memory and
//! paging, the heap, the PIC + PIT timer, keyboard input, and then starts
//! the NexOS shell.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod allocator;
mod apps;
// Architecture-specific modules: x86_64 lives in src/*.rs, the 32-bit (i686)
// port of the same modules in src/i686/. Everything else is shared.
#[cfg_attr(target_arch = "x86", path = "i686/boot.rs")]
mod boot;
mod console;
mod cpu;
mod fs;
#[cfg_attr(target_arch = "x86", path = "i686/gdt.rs")]
mod gdt;
mod input;
#[cfg_attr(target_arch = "x86", path = "i686/interrupts.rs")]
mod interrupts;
#[cfg_attr(target_arch = "x86", path = "i686/memory.rs")]
mod memory;
mod serial;
mod shell;
#[cfg_attr(target_arch = "x86", path = "i686/syscall.rs")]
mod syscall;
mod timer;
mod user;
mod vga;

use alloc::{boxed::Box, format, vec::Vec};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use vga::Color;

const MULTIBOOT2_MAGIC: u64 = 0x36d7_6289;

/// Entry from the boot stub: `usize` arguments so the same signature works
/// for the 64-bit (System V) and 32-bit (cdecl) boot stubs.
#[no_mangle]
pub extern "C" fn kernel_main(magic: usize, info_addr: usize) -> ! {
    let (magic, info_addr) = (magic as u64, info_addr as u64);
    serial::init();
    vga::WRITER.lock().clear();

    console::colored(Color::LightCyan, format_args!("NexOS v{} ({}) - booting\n", env!("CARGO_PKG_VERSION"), cpu::ARCH));
    if magic != MULTIBOOT2_MAGIC {
        panic!("not booted by a Multiboot2 loader (magic {:#x})", magic);
    }
    let info = unsafe { memory::parse_multiboot(info_addr) };
    ok!("boot loader: {}", info.loader_name());
    ok!("serial console on COM1 (everything on screen is mirrored here)");

    gdt::init();
    #[cfg(target_arch = "x86_64")]
    ok!("GDT loaded: kernel + user segments, TSS with double-fault stack");
    #[cfg(target_arch = "x86")]
    ok!("GDT loaded: flat 32-bit kernel + user segments, TSS for ring-3 entry");
    interrupts::init_idt();
    ok!("IDT loaded: breakpoint, page fault, GPF, double fault, syscall gate 0x80");

    memory::init(&info);
    ok!("memory: {} MiB usable RAM, frame allocator ready", info.usable_bytes() / (1024 * 1024));
    let pages = allocator::init().unwrap_or_else(|e| panic!("heap init failed: {}", e));
    ok!("paging: mapped {} pages for a 1 MiB heap at {:#x}", pages, memory::HEAP_START);

    let boxed = Box::new(41u64);
    let v: Vec<u64> = (1..=100).collect();
    let s = format!("{}", *boxed + 1);
    assert!(v.iter().sum::<u64>() == 5050 && s == "42");
    ok!("heap test: Box, Vec and String work");

    interrupts::init_pics();
    timer::init_pit();
    crate::cpu::interrupts::enable();
    ok!("PIC remapped, PIT timer at {} Hz, interrupts on", timer::HZ);
    ok!("input: PS/2 keyboard + serial (COM1) keyboard");

    crate::cpu::interrupts::int3();
    ok!("breakpoint exception handled, execution resumed");

    let files = fs::init();
    ok!("in-memory filesystem: {} files", files);
    ok!("user mode: {} programs registered, syscalls via int 0x80", user::APPS.len());
    let (in_store, installed) = apps::store::init();
    ok!("App Store: {} apps in the catalog, {} installed", in_store, installed);

    timer::boot_chime();
    console::colored(Color::LightCyan, format_args!("Welcome to NexOS.\n"));
    shell::run()
}

/// Recursion lock: a panic inside the panic handler goes straight to halt.
static PANICKING: AtomicBool = AtomicBool::new(false);

fn halt_loop() -> ! {
    loop {
        crate::cpu::interrupts::disable();
        crate::cpu::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::cpu::interrupts::disable();
    if PANICKING.swap(true, Ordering::SeqCst) {
        halt_loop();
    }
    // Avoid deadlocking on a console lock held when the panic hit.
    unsafe {
        serial::COM1.force_unlock();
        vga::WRITER.force_unlock();
    }
    console::colored(Color::Yellow, format_args!("\n*** KERNEL PANIC ***\n{}\n", info));
    halt_loop();
}
