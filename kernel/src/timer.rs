//! PIT timer (100 Hz) for uptime, the PC speaker (PIT channel 2) for chimes and
//! soundscapes, and the CMOS real-time clock for timestamps.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use crate::cpu::port::Port;

pub const HZ: u64 = 100;
const PIT_BASE_HZ: u32 = 1_193_182;

static TICKS: AtomicU64 = AtomicU64::new(0);
pub static MUTED: AtomicBool = AtomicBool::new(false);
/// True while a foreground `tone()` owns the speaker; the soundscape waits.
pub static FOREGROUND: AtomicBool = AtomicBool::new(false);
/// How many times PIT channel 2 has been programmed with a note since boot.
pub static NOTES: AtomicU32 = AtomicU32::new(0);

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
    crate::apps::soundscape::tick();
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
        crate::cpu::hlt();
    }
}

pub fn speaker_on(freq: u32) {
    NOTES.fetch_add(1, Ordering::Relaxed);
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

pub fn speaker_off() {
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
    FOREGROUND.store(true, Ordering::SeqCst);
    speaker_on(freq);
    sleep_ms(ms);
    speaker_off();
    FOREGROUND.store(false, Ordering::SeqCst);
}

/// A short, classic rising boot chime (C5 E5 G5 C6).
pub fn boot_chime() {
    for (f, ms) in [(523, 90), (659, 90), (784, 90), (1047, 180)] {
        tone(f, ms);
    }
}

/// A soft two-note hello when Insomnia opens.
pub fn boot_chime_soft() {
    tone(523, 80);
    tone(784, 140);
}

/// App Store: a bright rising arpeggio when an app finishes installing (C6 E6 G6 C7).
pub fn install_chime() {
    for (f, ms) in [(1047, 70), (1319, 70), (1568, 70), (2093, 220)] {
        tone(f, ms);
        sleep_ms(15);
    }
}

/// App Store: a soft falling pair when an app is removed (G5 then C5).
pub fn uninstall_chime() {
    tone(784, 120);
    sleep_ms(30);
    tone(523, 260);
}

/// Something went wrong: two short low buzzes.
pub fn error_tone() {
    tone(196, 140);
    sleep_ms(70);
    tone(196, 220);
}

/// Clock alarm: three rounds of a quick triple beep.
pub fn alarm_chime() {
    for _ in 0..3 {
        for _ in 0..3 {
            tone(1760, 90);
            sleep_ms(60);
        }
        sleep_ms(250);
    }
}

/// A tiny confirm click for saves.
pub fn ok_click() {
    tone(1319, 40);
}

/// A gentle falling goodnight chime (G5 E5 C5, then a long G4).
pub fn goodnight_chime() {
    for (f, ms) in [(784, 220), (659, 220), (523, 260), (392, 700)] {
        tone(f, ms);
        sleep_ms(60);
    }
}

fn cmos(reg: u8) -> u8 {
    unsafe {
        Port::<u8>::new(0x70).write(reg);
        Port::<u8>::new(0x71).read()
    }
}

/// Wall-clock hour and minute from the CMOS real-time clock.
pub fn rtc_hhmm() -> (u8, u8) {
    let (mut h, mut m, b) = crate::cpu::interrupts::without_interrupts(|| {
        let mut spins = 0;
        while cmos(0x0A) & 0x80 != 0 && spins < 100_000 {
            spins += 1; // wait out an RTC update in progress
        }
        (cmos(0x04), cmos(0x02), cmos(0x0B))
    });
    let pm = h & 0x80 != 0;
    h &= 0x7F;
    if b & 0x04 == 0 {
        h = (h & 0x0F) + (h >> 4) * 10;
        m = (m & 0x0F) + (m >> 4) * 10;
    }
    if b & 0x02 == 0 {
        h = (h % 12) + if pm { 12 } else { 0 }; // 12-hour RTC mode
    }
    (h % 24, m % 60)
}
