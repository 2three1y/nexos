//! Insomnia: a Looscid OS app for when you can't sleep.
//!
//! A calm night sky (moon + slowly twinkling stars), a sheep counter
//! (Space = one more sheep, with notes at milestones), and a short PC-speaker
//! lullaby that respects `mute`. Keyboard only: Space counts a sheep, M toggles
//! sound, T toggles twinkling, Esc (or Q) goes back to the shell.
//!
//! Accessibility: the picture is drawn on the VGA screen only; every message
//! (instructions, each sheep, milestones) is ALSO sent to the serial console
//! as plain text, so a screen reader hears what matters and none of the art.

use crate::vga::{self, put_at, SCREEN_COLS, SCREEN_ROWS};
use crate::{console, input, serial, timer};
use alloc::format;
use alloc::string::String;
use core::sync::atomic::Ordering;

const SKY: u8 = 0x0F; // bright white on black: high contrast
const DIM: u8 = 0x07; // light grey
const MOON: u8 = 0x0E; // bright yellow
const TEXT: u8 = 0x0F;
const ACCENT: u8 = 0x0B; // bright cyan

const MOON_ART: [&str; 7] = [
    "     _..._     ",
    "   .'  o  '.   ",
    "  /  O    o \\  ",
    " |  o   O    | ",
    "  \\    o   O/  ",
    "   '._ O _.'   ",
    "      ` `      ",
];

const SHEEP: [&str; 3] = [" .@@@@.  ", "(@@@@@@)o", "  || ||  "];
const BLANK: [&str; 3] = ["         ", "         ", "         "];
const FENCE_ROW: usize = 19;
const FENCE_COL: usize = 36;

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

fn say(text: &str) {
    // The message line on screen, and the same words on serial.
    vga::clear_row_attr(23, TEXT);
    put_at(23, 2, text, ACCENT);
    serial::write_str(text);
    serial::write_str("\n");
}

fn status(sheep: u32, muted: bool, twinkle: bool) {
    vga::clear_row_attr(22, TEXT);
    let s = format!(
        "Sheep counted: {:<6}  Sound: {:<3}  Twinkle: {:<3}",
        sheep,
        if muted { "off" } else { "on" },
        if twinkle { "on" } else { "off" }
    );
    put_at(22, 2, &s, TEXT);
}

fn milestone(n: u32) -> Option<String> {
    let s = match n {
        1 => "Sheep #1 clears the fence. Gold medal.",
        10 => "10 sheep. They have started a group chat.",
        25 => "25 sheep. The fence is filing a complaint.",
        50 => "50 sheep. Half of them are also awake.",
        100 => "100 sheep! The sheep are now counting you.",
        250 => "250 sheep. That's a whole wool startup.",
        404 => "Sheep #404 not found... it went to sleep. Maybe you should too.",
        500 => "500 sheep. Okay, legend. Try closing your eyes?",
        _ => return None,
    };
    Some(String::from(s))
}

fn draw_sky(rng: &mut Rng) {
    for r in 0..SCREEN_ROWS {
        vga::clear_row_attr(r, SKY);
    }
    for _ in 0..90 {
        let r = (rng.next() % 17) as usize;
        let c = (rng.next() % SCREEN_COLS as u32) as usize;
        let ch = match rng.next() % 6 { 0 => "*", 1 => "+", _ => "." };
        put_at(r, c, ch, if rng.next() % 3 == 0 { SKY } else { DIM });
    }
    for (i, line) in MOON_ART.iter().enumerate() {
        put_at(2 + i, 58, line, MOON);
    }
    put_at(0, 2, "Insomnia  -  a Looscid OS app", ACCENT);
    // The fence and the meadow.
    put_at(FENCE_ROW + 1, 0, &"_".repeat(SCREEN_COLS), DIM);
    put_at(FENCE_ROW - 1, FENCE_COL, "|-|-|", SKY);
    put_at(FENCE_ROW, FENCE_COL, "|-|-|", SKY);
    put_at(24, 2, "Space: count a sheep   M: sound on/off   T: twinkle on/off   Esc: back", DIM);
}

fn draw_sheep(row: usize, col: usize, art: &[&str; 3]) {
    for (i, line) in art.iter().enumerate() {
        put_at(row + i - 2, col, line, SKY);
    }
}

/// One sheep hops over the fence (a few frames, about 0.4 s).
fn hop() {
    let path: [(usize, usize); 6] = [(19, 22), (18, 27), (16, 32), (16, 38), (18, 43), (19, 48)];
    for (i, &(r, c)) in path.iter().enumerate() {
        draw_sheep(r, c, &SHEEP);
        put_at(FENCE_ROW - 1, FENCE_COL, "|-|-|", SKY);
        put_at(FENCE_ROW, FENCE_COL, "|-|-|", SKY);
        timer::sleep_ms(60);
        draw_sheep(r, c, &BLANK);
        if i == path.len() - 1 {
            draw_sheep(r, c, &SHEEP);
        }
    }
    put_at(FENCE_ROW - 1, FENCE_COL, "|-|-|", SKY);
    put_at(FENCE_ROW, FENCE_COL, "|-|-|", SKY);
}

fn lullaby() {
    // A soft, slow lullaby phrase (G4 E4 G4 E4 F4 D4 C4).
    for (f, ms) in [(392, 260), (330, 260), (392, 260), (330, 260), (349, 260), (294, 260), (262, 520)] {
        timer::tone(f, ms);
        timer::sleep_ms(30);
    }
}

pub fn run() {
    let mut rng = Rng(0x23E1_1D1A ^ (timer::ticks() as u32 | 1));
    let mut sheep: u32 = 0;
    let mut twinkle = true;
    vga::hide_cursor();
    draw_sky(&mut rng);
    status(sheep, timer::MUTED.load(Ordering::Relaxed), twinkle);
    serial::write_str("\n[Insomnia] A calm night sky with a full moon and stars, and a fence in a meadow.\n");
    serial::write_str("[Insomnia] Keys: Space counts a sheep, M turns sound on or off, T toggles twinkling, Esc goes back to the shell.\n");
    say("Can't sleep? Press Space to count sheep.");
    lullaby();

    let mut next_twinkle = timer::ticks() + 50;
    loop {
        x86_64::instructions::interrupts::disable();
        let key = input::pop();
        x86_64::instructions::interrupts::enable();
        match key {
            Some(b' ') => {
                sheep += 1;
                timer::tone(880, 25);
                hop();
                status(sheep, timer::MUTED.load(Ordering::Relaxed), twinkle);
                match milestone(sheep) {
                    Some(m) => say(&m),
                    None => say(&format!("Sheep {}.", sheep)),
                }
                if sheep == 404 {
                    lullaby();
                }
            }
            Some(b'm') | Some(b'M') => {
                let now = !timer::MUTED.load(Ordering::Relaxed);
                timer::MUTED.store(now, Ordering::Relaxed);
                status(sheep, now, twinkle);
                say(if now { "Sound off." } else { "Sound on." });
            }
            Some(b't') | Some(b'T') => {
                twinkle = !twinkle;
                status(sheep, timer::MUTED.load(Ordering::Relaxed), twinkle);
                say(if twinkle { "Stars twinkling." } else { "Stars still. No motion." });
            }
            Some(0x1b) | Some(b'q') | Some(b'Q') => break,
            Some(_) => {}
            None => {
                if twinkle && timer::ticks() >= next_twinkle {
                    // Gently swap one star for another.
                    let r = (rng.next() % 17) as usize;
                    let c = (rng.next() % 56) as usize;
                    let ch = if rng.next() % 2 == 0 { "." } else { " " };
                    put_at(r, c, ch, if rng.next() % 4 == 0 { SKY } else { DIM });
                    next_twinkle = timer::ticks() + 40;
                }
                x86_64::instructions::hlt();
            }
        }
    }

    vga::show_cursor();
    console::clear();
    crate::println!("Back from Insomnia. You counted {} sheep. Goodnight.", sheep);
}
