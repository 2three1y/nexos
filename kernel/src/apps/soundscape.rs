//! Soundscapes on the PC speaker (PIT channel 2), played in the background.
//!
//! The speaker has one square-wave voice and no volume knob, so each ambience
//! is a little step sequencer driven by the 100 Hz timer interrupt: a step is
//! "this note for N ticks" or "silence for N ticks". Rain is random short
//! clicks, Fan is a low humming pulse, Crickets are bursts of chirps, Old PC is
//! a hum with hard-drive seek clicks, and Lullaby is Brahms' lullaby.
//!
//! Tempo scales every step; Intensity (soft / medium / full) changes how busy
//! the pattern is, which is the closest the speaker gets to a volume control.
//! `mute` is honoured on every step: the pattern keeps its place silently.

use crate::timer;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use spin::Mutex;

pub const NAMES: [&str; 6] = ["nothing", "Rain", "Fan", "Crickets", "Old PC", "Lullaby"];
pub const ABOUT: [&str; 6] = [
    "",
    "soft random clicks, like rain on a window",
    "a low humming pulse",
    "little chirps in the dark",
    "hard-drive hum and seek clicks",
    "Brahms' lullaby, slow and quiet",
];
pub const INTENSITY_NAMES: [&str; 3] = ["soft", "medium", "full"];

static PATTERN: AtomicU8 = AtomicU8::new(0);
static TEMPO: AtomicU32 = AtomicU32::new(100);
static INTENSITY: AtomicU8 = AtomicU8::new(1);

pub fn pattern() -> usize { PATTERN.load(Ordering::Relaxed) as usize }
pub fn playing() -> &'static str { NAMES[pattern()] }
pub fn tempo() -> u32 { TEMPO.load(Ordering::Relaxed) }
pub fn intensity() -> &'static str { INTENSITY_NAMES[INTENSITY.load(Ordering::Relaxed) as usize] }

pub fn play(p: usize) {
    if p < NAMES.len() {
        PATTERN.store(p as u8, Ordering::SeqCst);
    }
}

pub fn stop() {
    PATTERN.store(0, Ordering::SeqCst);
    // Let the interrupt-side sequencer notice and switch the speaker off.
    timer::sleep_ms(30);
    timer::speaker_off();
}

/// Faster (+1) or slower (-1). Returns the new tempo in percent (50..=200).
pub fn change_tempo(dir: i32) -> u32 {
    let steps = [50u32, 75, 100, 125, 150, 200];
    let cur = tempo();
    let i = steps.iter().position(|&s| s == cur).unwrap_or(2) as i32;
    let j = (i + dir).clamp(0, steps.len() as i32 - 1) as usize;
    TEMPO.store(steps[j], Ordering::Relaxed);
    steps[j]
}

pub fn cycle_intensity() -> &'static str {
    let n = (INTENSITY.load(Ordering::Relaxed) + 1) % 3;
    INTENSITY.store(n, Ordering::Relaxed);
    INTENSITY_NAMES[n as usize]
}

struct Seq {
    pat: u8,
    left: u32,
    idx: u32,
    rng: u32,
    on: bool,
}

static SEQ: Mutex<Seq> = Mutex::new(Seq { pat: 0, left: 0, idx: 0, rng: 0x5EED_2311, on: false });

// Brahms' Lullaby: (frequency in Hz, length in eighths).
const LULLABY: [(u32, u32); 27] = [
    (330, 1), (330, 1), (392, 3), (330, 1), (330, 1), (392, 3),
    (330, 1), (392, 1), (523, 2), (494, 2), (440, 2), (440, 2), (392, 4),
    (294, 1), (330, 1), (349, 2), (294, 2), (294, 1), (330, 1), (349, 4),
    (294, 1), (349, 1), (494, 1), (440, 2), (392, 2), (494, 2), (523, 6),
];

impl Seq {
    fn rand(&mut self, lo: u32, hi: u32) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        lo + self.rng % (hi - lo + 1)
    }

    /// The next step: (frequency, or 0 for silence; length in ticks).
    fn step(&mut self, pat: u8, inten: u8) -> (u32, u32) {
        let i = self.idx;
        self.idx = self.idx.wrapping_add(1);
        match pat {
            1 => {
                // Rain: a 10 ms click, then a random gap.
                if i % 2 == 0 {
                    (self.rand(1800, 5800), 1)
                } else {
                    match inten { 0 => (0, self.rand(4, 18)), 1 => (0, self.rand(1, 8)), _ => (0, self.rand(1, 3)) }
                }
            }
            2 => {
                // Fan: a low hum that breathes.
                if i % 2 == 0 {
                    let len = [12, 22, 40][inten as usize];
                    (self.rand(84, 92), len)
                } else {
                    (0, [6, 3, 1][inten as usize])
                }
            }
            3 => {
                // Crickets: four quick chirps, then a pause.
                let k = i % 9;
                if k < 8 {
                    if k % 2 == 0 { (self.rand(4200, 4500), 2) } else { (0, 2) }
                } else {
                    match inten { 0 => (0, self.rand(120, 250)), 1 => (0, self.rand(60, 150)), _ => (0, self.rand(25, 70)) }
                }
            }
            4 => {
                // Old PC: drive hum with the odd burst of seek clicks.
                let seek_chance = [12, 25, 45][inten as usize];
                if i % 2 == 0 {
                    if self.rand(0, 99) < seek_chance { (self.rand(2400, 3400), 1) } else { (self.rand(118, 122), self.rand(6, 20)) }
                } else {
                    (0, 1)
                }
            }
            5 => {
                // Lullaby: each note, then a small breath; a long rest at the end.
                let n = (i / 2) as usize;
                if n >= LULLABY.len() {
                    self.idx = 0;
                    return (0, 120);
                }
                let (f, eighths) = LULLABY[n];
                let full = eighths * 18;
                let pct = [60, 85, 100][inten as usize];
                if i % 2 == 0 { (f, (full * pct / 100).max(1)) } else { (0, (full - full * pct / 100).max(2)) }
            }
            _ => (0, 10),
        }
    }
}

/// Called from the timer interrupt, 100 times a second.
pub fn tick() {
    let pat = PATTERN.load(Ordering::Relaxed);
    let mut s = match SEQ.try_lock() {
        Some(s) => s,
        None => return,
    };
    if pat != s.pat {
        s.pat = pat;
        s.left = 0;
        s.idx = 0;
        if s.on {
            timer::speaker_off();
            s.on = false;
        }
    }
    if pat == 0 {
        return;
    }
    if timer::FOREGROUND.load(Ordering::Relaxed) {
        // A chime or a sheep click owns the speaker right now.
        s.on = false;
        return;
    }
    if s.left > 0 {
        s.left -= 1;
        return;
    }
    let inten = INTENSITY.load(Ordering::Relaxed).min(2);
    let (freq, ticks) = s.step(pat, inten);
    let muted = timer::MUTED.load(Ordering::Relaxed);
    if freq > 0 && !muted {
        timer::speaker_on(freq);
        s.on = true;
    } else if s.on {
        timer::speaker_off();
        s.on = false;
    }
    let scaled = (ticks * 100 / TEMPO.load(Ordering::Relaxed).max(1)).max(1);
    s.left = scaled - 1;
}
