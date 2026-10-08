//! `hello` — the first NexOS userland program.
//!
//! Runs in ring 3 with no access to kernel memory or I/O ports. Everything it
//! does goes through NexOS system calls (`int 0x80`, number in RAX, or EAX on i686).

#![no_std]
#![no_main]

use core::fmt::{self, Write};
use nexos::syscall;

const SYS_EXIT: u64 = 0;
const SYS_WRITE: u64 = 1;
const SYS_UPTIME_MS: u64 = 2;
const SYS_GETPID: u64 = 3;

fn write(s: &str) {
    unsafe { syscall(SYS_WRITE, s.as_ptr() as u64, s.len() as u64) };
}

fn exit(code: i64) -> ! {
    unsafe { syscall(SYS_EXIT, code as u64, 0) };
    loop {}
}

/// Small fixed buffer so we can format numbers without an allocator.
struct Buf {
    data: [u8; 128],
    len: usize,
}

impl Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let n = s.len().min(self.data.len() - self.len);
        self.data[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    write("Hello from ring 3! I'm the first NexOS userland program.\n");
    let pid = unsafe { syscall(SYS_GETPID, 0, 0) };
    let up = unsafe { syscall(SYS_UPTIME_MS, 0, 0) };
    let mut b = Buf { data: [0; 128], len: 0 };
    let _ = write!(b, "pid {} - the kernel says it has been up {} ms.\n", pid, up);
    write(core::str::from_utf8(&b.data[..b.len]).unwrap_or("?\n"));
    write("Goodbye, handing control back to the NexOS kernel.\n");
    exit(0)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    write("hello: panic\n");
    exit(1)
}
