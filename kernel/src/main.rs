//! NexOS — the kernel of Looscid OS.
//!
//! Boot path: GRUB (Multiboot2) -> boot.rs (32-bit stub, long mode) ->
//! `kernel_main`, which brings up serial + VGA, GDT/TSS, IDT, memory and
//! paging, the heap, the PIC + PIT timer, keyboard input, and then starts
//! the Looscid shell.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod allocator;
mod apps;
mod boot;
mod console;
mod fs;
mod gdt;
mod input;
mod interrupts;
mod memory;
mod serial;
mod shell;
mod syscall;
mod timer;
mod user;
mod vga;

use alloc::{boxed::Box, format, vec::Vec};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use vga::Color;

const MULTIBOOT2_MAGIC: u64 = 0x36d7_6289;

#[no_mangle]
pub extern "C" fn kernel_main(magic: u64, info_addr: u64) -> ! {
    serial::init();
    vga::WRITER.lock().clear();

    console::colored(Color::LightCyan, format_args!("NexOS kernel v{} (x86_64) - booting Looscid OS\n", env!("CARGO_PKG_VERSION")));
    if magic != MULTIBOOT2_MAGIC {
        panic!("not booted by a Multiboot2 loader (magic {:#x})", magic);
    }
    let info = unsafe { memory::parse_multiboot(info_addr) };
    ok!("boot loader: {}", info.loader_name());
    ok!("serial console on COM1 (everything on screen is mirrored here)");

    gdt::init();
    ok!("GDT loaded: kernel + user segments, TSS with double-fault stack");
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
    x86_64::instructions::interrupts::enable();
    ok!("PIC remapped, PIT timer at {} Hz, interrupts on", timer::HZ);
    ok!("input: PS/2 keyboard + serial (COM1) keyboard");

    x86_64::instructions::interrupts::int3();
    ok!("breakpoint exception handled, execution resumed");

    let files = fs::init();
    ok!("in-memory filesystem: {} files", files);
    ok!("user mode: {} programs registered, syscalls via int 0x80", user::APPS.len());
    let (in_store, installed) = apps::store::init();
    ok!("App Store: {} apps in the catalog, {} installed", in_store, installed);

    timer::boot_chime();
    console::colored(Color::LightCyan, format_args!("Welcome to Looscid OS.\n"));
    shell::run()
}

/// Recursion lock: a panic inside the panic handler goes straight to halt.
static PANICKING: AtomicBool = AtomicBool::new(false);

fn halt_loop() -> ! {
    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
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
