//! Insomnia: a Looscid OS app for when you can't sleep.
//!
//! A small menu: 1 Sheep, 2 Thoughts, 3 Sounds, 4 Goodnight.
//! - Sheep: a calm night sky (moon + slowly twinkling stars) and a sheep
//!   counter (Space = one more sheep, with notes at milestones). M toggles
//!   sound, T toggles twinkling, Esc goes back to the menu.
//! - Thoughts: the 4am notepad, saved to thoughts.txt (see thoughts.rs).
//! - Sounds: PC-speaker soundscapes that keep playing in the background
//!   while you count sheep or write (see soundscape.rs).
//! - Goodnight: a gentle chime, a calm screen, then back to the shell.
//!
//! Accessibility: the picture is drawn on the VGA screen only; every message
//! (instructions, each sheep, milestones) is ALSO sent to the serial console
//! as plain text, so a screen reader hears what matters and none of the art.

use crate::vga::{self, put_at, SCREEN_COLS, SCREEN_ROWS};
use super::{soundscape, thoughts};
use crate::{console, input, serial, timer};
use alloc::format;
use alloc::string::String;
use core::sync::atomic::Ordering;

const SKY: u8 = 0x0F; // bright white on black: high contrast
pub const DIM: u8 = 0x07; // light grey
const MOON: u8 = 0x0E; // bright yellow
pub const TEXT: u8 = 0x0F;
pub const ACCENT: u8 = 0x0B; // bright cyan

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

pub fn say(text: &str) {
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
    put_at(24, 2, "Space: count a sheep   M: sound on/off   T: twinkle on/off   Esc: menu", DIM);
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

fn sheep_screen(sheep: &mut u32, twinkle: &mut bool, first: bool) {
    let mut rng = Rng(0x23E1_1D1A ^ (timer::ticks() as u32 | 1));
    draw_sky(&mut rng);
    status(*sheep, timer::MUTED.load(Ordering::Relaxed), *twinkle);
    serial::write_str("\n[Sheep] A calm night sky with a full moon and stars, and a fence in a meadow.\n");
    serial::write_str("[Sheep] Keys: Space counts a sheep, M turns sound on or off, T toggles twinkling, Esc goes back to the menu.\n");
    if *sheep == 0 {
        say("Can't sleep? Press Space to count sheep.");
    } else {
        say(&format!("Welcome back. {} sheep so far. Press Space for the next one.", sheep));
    }
    if first && soundscape::pattern() == 0 {
        lullaby();
    }

    let mut next_twinkle = timer::ticks() + 50;
    loop {
        x86_64::instructions::interrupts::disable();
        let key = input::pop();
        x86_64::instructions::interrupts::enable();
        match key {
            Some(b' ') => {
                *sheep += 1;
                timer::tone(880, 25);
                hop();
                status(*sheep, timer::MUTED.load(Ordering::Relaxed), *twinkle);
                match milestone(*sheep) {
                    Some(m) => say(&m),
                    None => say(&format!("Sheep {}.", sheep)),
                }
                if *sheep == 404 {
                    lullaby();
                }
            }
            Some(b'm') | Some(b'M') => {
                let now = toggle_mute();
                status(*sheep, now, *twinkle);
                say(if now { "Sound off." } else { "Sound on." });
            }
            Some(b't') | Some(b'T') => {
                *twinkle = !*twinkle;
                status(*sheep, timer::MUTED.load(Ordering::Relaxed), *twinkle);
                say(if *twinkle { "Stars twinkling." } else { "Stars still. No motion." });
            }
            Some(0x1b) | Some(b'q') | Some(b'Q') => break,
            Some(_) => {}
            None => {
                if *twinkle && timer::ticks() >= next_twinkle {
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
}

fn toggle_mute() -> bool {
    let now = !timer::MUTED.load(Ordering::Relaxed);
    timer::MUTED.store(now, Ordering::Relaxed);
    if now {
        timer::speaker_off();
    }
    now
}

fn clear_screen() {
    for r in 0..SCREEN_ROWS {
        vga::clear_row_attr(r, TEXT);
    }
}

fn sound_word() -> &'static str {
    if timer::MUTED.load(Ordering::Relaxed) { "off" } else { "on" }
}

fn menu_status(sheep: u32) -> String {
    format!(
        "Sheep counted: {}   Thoughts saved: {}   Sound: {}   Playing: {}",
        sheep,
        thoughts::count(),
        sound_word(),
        soundscape::playing()
    )
}

const MENU: [&str; 4] = [
    "1  Sheep       count sheep over the fence",
    "2  Thoughts    a 4am notepad, saved to thoughts.txt",
    "3  Sounds      rain, fan, crickets, old PC, lullaby",
    "4  Goodnight   a soft chime, then back to the shell",
];

fn draw_menu(sheep: u32) {
    let mut rng = Rng(0x4A11_0C42);
    clear_screen();
    for _ in 0..40 {
        let r = 1 + (rng.next() % 19) as usize;
        let c = (rng.next() % SCREEN_COLS as u32) as usize;
        if (5..16).contains(&r) && c < 58 {
            continue; // keep the menu area clear and easy to read
        }
        put_at(r, c, if rng.next() % 4 == 0 { "*" } else { "." }, DIM);
    }
    for (i, line) in MOON_ART.iter().enumerate() {
        put_at(2 + i, 60, line, MOON);
    }
    put_at(0, 2, "Insomnia  -  a Looscid OS app", ACCENT);
    put_at(6, 6, "What do you need tonight?", TEXT);
    for (i, line) in MENU.iter().enumerate() {
        put_at(8 + i * 2, 8, line, TEXT);
    }
    vga::clear_row_attr(22, TEXT);
    put_at(22, 2, &menu_status(sheep), TEXT);
    put_at(24, 2, "1-4: choose   M: sound on/off   Esc: back to the shell", DIM);
}

fn announce_menu(sheep: u32) {
    serial::write_str("\n[Insomnia menu] What do you need tonight? 1 Sheep, 2 Thoughts, 3 Sounds, 4 Goodnight. M turns sound on or off. Esc goes back to the shell.\n");
    serial::write_str(&format!(
        "[Insomnia menu] Sheep counted {}. Thoughts saved {}. Sound {}. Playing {}.\n",
        sheep,
        thoughts::count(),
        sound_word(),
        soundscape::playing()
    ));
}

fn sounds_status() -> String {
    format!(
        "Playing: {:<9} Tempo: {}%   Intensity: {:<7} Sound: {}",
        soundscape::playing(),
        soundscape::tempo(),
        soundscape::intensity(),
        sound_word()
    )
}

fn draw_sounds() {
    clear_screen();
    put_at(0, 2, "Sounds  -  soundscapes on the PC speaker", ACCENT);
    put_at(1, 2, &"-".repeat(SCREEN_COLS - 4), DIM);
    let cur = soundscape::pattern();
    for p in 1..soundscape::NAMES.len() {
        let row = 2 + p * 2;
        let mark = if p == cur { ">" } else { " " };
        put_at(row, 4, mark, ACCENT);
        put_at(row, 6, &format!("{}  {:<10} {}", p, soundscape::NAMES[p], soundscape::ABOUT[p]), TEXT);
    }
    put_at(14, 4, if cur == 0 { ">" } else { " " }, ACCENT);
    put_at(14, 6, "0  Stop", TEXT);
    vga::clear_row_attr(17, TEXT);
    put_at(17, 4, &sounds_status(), TEXT);
    put_at(19, 4, "The PC speaker has one voice and no volume knob, so Intensity", DIM);
    put_at(20, 4, "changes how busy the sound is. It keeps playing on other screens.", DIM);
    put_at(24, 2, "1-5: play  0: stop  F/S: faster/slower  I: intensity  M: mute  Esc: menu", DIM);
}

fn announce_playing(entering: bool) {
    let p = soundscape::pattern();
    let msg = if p == 0 {
        format!("{} Tempo {} percent, intensity {}.", if entering { "Nothing playing." } else { "Stopped." }, soundscape::tempo(), soundscape::intensity())
    } else if timer::MUTED.load(Ordering::Relaxed) {
        format!("Now playing {}, but sound is off. Press M to hear it.", soundscape::playing())
    } else {
        format!(
            "Now playing {}. Tempo {} percent, intensity {}.",
            soundscape::playing(),
            soundscape::tempo(),
            soundscape::intensity()
        )
    };
    say(&msg);
}

fn sounds_screen() {
    draw_sounds();
    serial::write_str("\n[Sounds] Soundscapes on the PC speaker. 1 Rain, 2 Fan, 3 Crickets, 4 Old PC, 5 Lullaby, 0 Stop.\n");
    serial::write_str("[Sounds] F faster, S slower, I changes intensity, M turns sound on or off, Esc goes back to the menu. The sound keeps playing on other screens.\n");
    announce_playing(true);
    loop {
        let b = input::read_key();
        match b {
            b'1'..=b'5' => {
                soundscape::play((b - b'0') as usize);
                draw_sounds();
                announce_playing(false);
            }
            b'0' | b' ' => {
                soundscape::stop();
                draw_sounds();
                announce_playing(false);
            }
            b'f' | b'F' | b'+' | b'=' => {
                let t = soundscape::change_tempo(1);
                draw_sounds();
                say(&format!("Tempo {} percent.", t));
            }
            b's' | b'S' | b'-' | b'_' => {
                let t = soundscape::change_tempo(-1);
                draw_sounds();
                say(&format!("Tempo {} percent.", t));
            }
            b'i' | b'I' => {
                let i = soundscape::cycle_intensity();
                draw_sounds();
                say(&format!("Intensity {}.", i));
            }
            b'm' | b'M' => {
                let now = toggle_mute();
                draw_sounds();
                say(if now { "Sound off. The pattern keeps its place silently." } else { "Sound on." });
            }
            0x1b | b'q' | b'Q' => break,
            _ => {}
        }
    }
}

fn goodnight() {
    soundscape::stop();
    clear_screen();
    let a = "Goodnight, Hasan.";
    let b = "It's now safe to turn off your brain.";
    put_at(10, (SCREEN_COLS - a.len()) / 2, a, MOON);
    put_at(12, (SCREEN_COLS - b.len()) / 2, b, MOON);
    let c = "Returning to the shell in 10 seconds, or press any key.";
    put_at(22, (SCREEN_COLS - c.len()) / 2, c, DIM);
    serial::write_str("\n[Goodnight] Goodnight, Hasan. It's now safe to turn off your brain.\n");
    serial::write_str("[Goodnight] Returning to the shell in 10 seconds, or press any key.\n");
    timer::goodnight_chime();
    // Drop keys typed during the chime, then wait for a key or 10 seconds.
    x86_64::instructions::interrupts::without_interrupts(|| while input::pop().is_some() {});
    let end = timer::ticks() + 10 * timer::HZ;
    while timer::ticks() < end {
        x86_64::instructions::interrupts::disable();
        let k = input::pop();
        x86_64::instructions::interrupts::enable();
        if k.is_some() {
            break;
        }
        x86_64::instructions::hlt();
    }
}

pub fn run() {
    let mut sheep: u32 = 0;
    let mut twinkle = true;
    let mut first_sheep = true;
    vga::hide_cursor();
    timer::boot_chime_soft();
    draw_menu(sheep);
    announce_menu(sheep);
    let mut said_goodnight = false;
    loop {
        let b = input::read_key();
        match b {
            b'1' => {
                sheep_screen(&mut sheep, &mut twinkle, first_sheep);
                first_sheep = false;
            }
            b'2' => thoughts::run(),
            b'3' => sounds_screen(),
            b'4' => {
                goodnight();
                said_goodnight = true;
                break;
            }
            b'm' | b'M' => {
                let now = toggle_mute();
                draw_menu(sheep);
                say(if now { "Sound off." } else { "Sound on." });
                continue;
            }
            0x1b | b'q' | b'Q' => break,
            _ => continue,
        }
        draw_menu(sheep);
        announce_menu(sheep);
    }

    soundscape::stop();
    vga::show_cursor();
    console::clear();
    if said_goodnight {
        crate::println!("Back from Insomnia. You counted {} sheep. Sleep well, Hasan.", sheep);
    } else {
        crate::println!("Back from Insomnia. You counted {} sheep. Goodnight.", sheep);
    }
}
