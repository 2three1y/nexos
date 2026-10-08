//! PIT timer (100 Hz) for uptime, and the PC speaker for the boot chime.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use x86_64::instructions::port::Port;

pub const HZ: u64 = 100;
const PIT_BASE_HZ: u32 = 1_193_182;

static TICKS: AtomicU64 = AtomicU64::new(0);
pub static MUTED: AtomicBool = AtomicBool::new(false);

pub fn init_pit() {
    let divisor = (PIT_BASE_HZ / HZ as u32) as u16;
    unsafe {
        Port::<u8>::new(0x43).write(0x36); // channel 0, lo/hi, mode 3
        Port::<u8>::new(0x40).write((divisor & 0xFF) as u8);
        Port::<u8>::new(0x40).write((divisor >> 8) as u8);
    }
}

pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn uptime_ms() -> u64 {
    ticks() * 1000 / HZ
}

/// Sleep with interrupts enabled (the CPU halts between ticks).
pub fn sleep_ms(ms: u64) {
    let end = ticks() + (ms * HZ).div_ceil(1000);
    while ticks() < end {
        x86_64::instructions::hlt();
    }
}

fn speaker_on(freq: u32) {
    let div = (PIT_BASE_HZ / freq) as u16;
    unsafe {
        Port::<u8>::new(0x43).write(0xB6); // channel 2, square wave
        Port::<u8>::new(0x42).write((div & 0xFF) as u8);
        Port::<u8>::new(0x42).write((div >> 8) as u8);
        let mut ctl = Port::<u8>::new(0x61);
        let v = ctl.read();
        ctl.write(v | 3);
    }
}

fn speaker_off() {
    unsafe {
        let mut ctl = Port::<u8>::new(0x61);
        let v = ctl.read();
        ctl.write(v & !3);
    }
}

pub fn tone(freq: u32, ms: u64) {
    if MUTED.load(Ordering::Relaxed) {
        return;
    }
    speaker_on(freq);
    sleep_ms(ms);
    speaker_off();
}

/// A short, classic rising boot chime (C5 E5 G5 C6).
pub fn boot_chime() {
    for (f, ms) in [(523, 90), (659, 90), (784, 90), (1047, 180)] {
        tone(f, ms);
    }
}
