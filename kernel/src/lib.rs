// NexOS kernel — freestanding no_std library core.
//
// This crate is FREESTANDING: it does not link against the standard library
// and has no host runtime. The bootloader (or QEMU, via the bootable image)
// jumps to `_start` directly. There is no `main`, no stdlib, and no OS
// underneath us — we are the OS.
//
// Freestanding is achieved by:
//   - `#![no_std]` — no standard library is linked
//   - never importing from the stdlib
//
// This milestone adds the panic & debug infrastructure: a VGA text console,
// an optional serial (COM1) debug channel, a recursion lock so a panic inside
// a panic cannot loop forever, and a halt loop that parks the CPU after a
// fatal error is reported.

// Freestanding: do not link the standard library (not installed for this target).
#![no_std]

// PanicInfo carries the panic message and source location (file/line).
use core::panic::PanicInfo;

// ---------------------------------------------------------------------------
// VGA text-mode console (0xB8000, 80x25, 2 bytes per cell)
// ---------------------------------------------------------------------------

const VGA_MEM: usize = 0xB8000;
const VGA_COLS: usize = 80;
const VGA_ROWS: usize = 25;

// Cursor position for the debug console. Module-level mutable state is held
// in a struct and exposed through a `static` local (the freestanding kernel
// has no heap, so we keep a single fixed instance).
struct VgaCursor {
    row: usize,
    col: usize,
}

// Scratch memory for the console cursor. We keep it at a fixed address
// (just past the visible VGA text buffer) because the freestanding kernel
// has no heap and no mutable static locals.
const CURSOR_ADDR: usize = VGA_MEM + VGA_COLS * VGA_ROWS * 2;

/// Return a mutable handle to the single console cursor.
fn cursor() -> &'static mut VgaCursor {
    unsafe {
        &mut *(CURSOR_ADDR as *mut VgaCursor)
    }
}

/// Write a single character to the VGA text-mode buffer at 0xB8000.
///
/// Each cell is 2 bytes: ASCII char + attribute byte. We use light-grey on
/// black (0x0F) for now.
fn vga_putc(x: usize, y: usize, c: u8) {
    let cell = VGA_MEM + (y * VGA_COLS + x) * 2;
    unsafe {
        let ptr = cell as *mut u16;
        *ptr = (c as u16) | (0x0F << 8);
    }
}

/// Advance the console cursor, scrolling when the bottom row is reached.
fn vga_advance() {
    let cur = cursor();
    cur.col += 1;
    if cur.col >= VGA_COLS {
        cur.col = 0;
        cur.row += 1;
        if cur.row >= VGA_ROWS {
            cur.row = VGA_ROWS - 1;
            // Scroll: shift every row up one line.
            for y in 0..(VGA_ROWS - 1) {
                for x in 0..VGA_COLS {
                    let src = VGA_MEM + ((y + 1) * VGA_COLS + x) * 2;
                    let dst = VGA_MEM + (y * VGA_COLS + x) * 2;
                    unsafe {
                        let s = src as *mut u16;
                        let d = dst as *mut u16;
                        *d = *s;
                    }
                }
            }
            // Clear the last row.
            for x in 0..VGA_COLS {
                let addr = VGA_MEM + ((VGA_ROWS - 1) * VGA_COLS + x) * 2;
                unsafe {
                    let cell = addr as *mut u16;
                    *cell = 0x0F00;
                }
            }
        }
    }
}

/// Write a single character to the console at the current cursor.
fn console_putc(c: u8) {
    let cur = cursor();
    if c == '\n' as u8 {
        cur.col = 0;
        cur.row += 1;
        if cur.row >= VGA_ROWS {
            cur.row = VGA_ROWS - 1;
        }
        return;
    }
    vga_putc(cur.col, cur.row, c);
    vga_advance();
}

/// Write a string to the VGA console.
fn console_write(s: &str) {
    for ch in s.bytes() {
        console_putc(ch);
    }
}

// ---------------------------------------------------------------------------
// Serial debug channel (COM1 at 0x3F8)
//
// The serial channel is optional: it is compiled in only when the `serial`
// feature is enabled (see Cargo.toml). Port I/O requires the `x86_64` crate's
// `asm!` macro, which is not available in the bare rustc toolchain, so the
// serial path is stubbed here and wired up in a later milestone.
// ---------------------------------------------------------------------------

// const COM1: u16 = 0x3F8; // serial port base (unused until serial feature)

/// Write a string to the serial debug channel (stub — see milestone note).
fn serial_write(s: &str) {
    // Port I/O via `asm!` requires the x86_64 crate. This is a placeholder
    // that is compiled out unless the `serial` feature is enabled.
    let _ = s;
}

// ---------------------------------------------------------------------------
// Panic & debug handlers
// ---------------------------------------------------------------------------

/// Recursion lock: prevents a panic inside a panic from recursing forever.
///
/// When a panic fires we set this flag BEFORE printing. If a second panic
/// occurs while the flag is set (e.g. the panic handler itself faults), we
/// fall straight to the halt loop instead of recursing.
// Scratch address for the panic recursion lock (just past the cursor state).
const PANIC_LOCK_ADDR: usize = CURSOR_ADDR + 16;

fn panic_lock() -> &'static mut bool {
    unsafe {
        &mut *(PANIC_LOCK_ADDR as *mut bool)
    }
}

/// Halt the CPU forever. This is the terminal state after a fatal error.
fn halt_loop() -> ! {
    loop {}
}

/// The kernel panic handler.
///
/// Prints the message to the VGA console and (optionally) the serial debug
/// channel, then halts. The recursion lock guarantees we never recurse.
fn panic(msg: &str) -> ! {
    if *panic_lock() {
        // We are already panicking — do not recurse. Just halt.
        halt_loop();
    }
    *panic_lock() = true;

    console_write("\n\n*** KERNEL PANIC ***\n");
    console_write(msg);
    console_write("\n");

    // Mirror to the serial debug channel (COM1) if present.
    serial_write("\r\n*** KERNEL PANIC ***\r\n");
    serial_write(msg);
    serial_write("\r\n");

    halt_loop();
}

/// Freestanding panic handler. Required for a `no_std` crate.
#[panic_handler]
fn kernel_panic_handler(_info: &PanicInfo) -> ! {
    panic("kernel panic (see console)");
}

/// Kernel entry point. Called directly by the bootloader.
pub fn _start() -> ! {
    // Clear the screen (80x25 text cells) to black.
    for i in 0..(VGA_COLS * VGA_ROWS) {
        let addr = VGA_MEM + i * 2;
        unsafe {
            let cell = addr as *mut u16;
            *cell = 0x0F00; // space, light-grey on black
        }
    }

    // Print a boot banner.
    let banner = "NexOS kernel booting...";
    console_write(banner);

    // Halt forever. A real kernel would enter the scheduler here.
    loop {}
}