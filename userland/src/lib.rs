//! `nexos` — the app API for ring-3 NexOS programs.
//!
//! A tiny, allocation-free layer over the NexOS system calls (`int 0x80`) so
//! apps can print, read keys and lines, play tones and keep time. The same
//! calls (write, read_key, beep, sleep, uptime, exit) are the contract the
//! Linux edition will provide, so apps written against this API port over.
//! See docs/APPS.md.

#![no_std]

use core::arch::asm;
use core::fmt::{self, Write};

pub mod sys {
    pub const EXIT: u64 = 0;
    pub const WRITE: u64 = 1;
    pub const UPTIME_MS: u64 = 2;
    pub const GETPID: u64 = 3;
    pub const READ_KEY: u64 = 4;
    pub const BEEP: u64 = 5;
    pub const SLEEP_MS: u64 = 6;
}

/// Raw system call: number in RAX, arguments in RDI and RSI, result in RAX.
#[inline(always)]
pub unsafe fn syscall(nr: u64, a1: u64, a2: u64) -> i64 {
    let ret: i64;
    asm!("int 0x80", inlateout("rax") nr as i64 => ret, in("rdi") a1, in("rsi") a2, options(nostack));
    ret
}

pub fn write(s: &str) {
    for chunk in s.as_bytes().chunks(4096) {
        unsafe { syscall(sys::WRITE, chunk.as_ptr() as u64, chunk.len() as u64) };
    }
}

pub fn exit(code: i64) -> ! {
    unsafe { syscall(sys::EXIT, code as u64, 0) };
    loop {}
}

pub fn uptime_ms() -> u64 {
    unsafe { syscall(sys::UPTIME_MS, 0, 0) as u64 }
}

/// Wait for one key. Enter is b'\n', Backspace 8, Esc 27.
pub fn read_key() -> u8 {
    unsafe { syscall(sys::READ_KEY, 0, 0) as u8 }
}

/// Play a tone on the PC speaker (silent if the user muted sound).
pub fn beep(freq_hz: u32, ms: u64) {
    unsafe { syscall(sys::BEEP, freq_hz as u64, ms) };
}

pub fn sleep_ms(ms: u64) {
    unsafe { syscall(sys::SLEEP_MS, ms, 0) };
}

/// Fixed-size text buffer so programs can format without an allocator.
pub struct Buf<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buf<N> {
    pub const fn new() -> Self {
        Buf { data: [0; N], len: 0 }
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
    pub fn push(&mut self, b: u8) -> bool {
        if self.len < N {
            self.data[self.len] = b;
            self.len += 1;
            true
        } else {
            false
        }
    }
    pub fn pop(&mut self) -> bool {
        if self.len > 0 {
            self.len -= 1;
            true
        } else {
            false
        }
    }
}

impl<const N: usize> Write for Buf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            self.push(b);
        }
        Ok(())
    }
}

/// Print formatted text (up to 256 bytes per call).
pub fn print_fmt(args: fmt::Arguments) {
    let mut b: Buf<256> = Buf::new();
    let _ = b.write_fmt(args);
    write(b.as_str());
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print_fmt(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::write("\n"));
    ($($arg:tt)*) => ({ $crate::print_fmt(format_args!($($arg)*)); $crate::write("\n"); });
}

/// Read a line with echo into `buf`. Returns false if Esc was pressed.
pub fn read_line<const N: usize>(prompt: &str, buf: &mut Buf<N>) -> bool {
    write(prompt);
    buf.clear();
    loop {
        match read_key() {
            b'\n' => {
                write("\n");
                return true;
            }
            27 => {
                write("\n");
                return false;
            }
            8 => {
                if buf.pop() {
                    write("\x08");
                }
            }
            b @ 0x20..=0x7e => {
                if buf.push(b) {
                    let s = [b];
                    write(core::str::from_utf8(&s).unwrap_or(""));
                }
            }
            _ => {}
        }
    }
}

/// Small xorshift random numbers, seeded from timing.
pub struct Rng(u64);

impl Rng {
    pub fn seeded() -> Self {
        Rng(uptime_ms().wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn reseed(&mut self, extra: u64) {
        self.0 ^= extra.wrapping_mul(0xD6E8_FEB8_6659_FD93);
        if self.0 == 0 {
            self.0 = 1;
        }
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// A number from `lo` to `hi`, inclusive.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
}

/// Standard opening lines, matching the native apps.
pub fn title(name: &str, version: &str, blurb: &str) {
    println!();
    println!("{} {}", name, version);
    println!("{}", blurb);
    write("Type h for help, q to quit.\n");
}
