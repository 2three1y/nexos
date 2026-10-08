//! 4am Thoughts: a keyboard-only notepad inside Insomnia.
//!
//! Type a thought and press Enter: it is appended to `thoughts.txt` in the
//! in-memory filesystem with the time from the real-time clock, so
//! `cat thoughts.txt` in the shell shows them. Tab reads every saved thought
//! back on the serial console; Esc returns to the Insomnia menu. What you type
//! is echoed to serial exactly like the shell, so a screen reader follows along.

use super::insomnia::{say, ACCENT, DIM, TEXT};
use crate::vga::{self, put_at, SCREEN_COLS};
use crate::{fs, input, serial, timer};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub const FILE: &str = "thoughts.txt";
const MAX: usize = 200;
const INPUT_ROW: usize = 21;

pub fn count() -> usize {
    fs::read(FILE).map(|d| d.lines().filter(|l| !l.trim().is_empty()).count()).unwrap_or(0)
}

fn draw(line: &str) {
    for r in 0..25 {
        vga::clear_row_attr(r, TEXT);
    }
    put_at(0, 2, "4am Thoughts  -  saved to thoughts.txt", ACCENT);
    put_at(1, 2, &"-".repeat(SCREEN_COLS - 4), DIM);
    let saved = fs::read(FILE).unwrap_or_default();
    let lines: Vec<&str> = saved.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        put_at(3, 4, "No thoughts yet. Whatever is keeping you up, write it down here.", DIM);
    }
    let first = lines.len().saturating_sub(16);
    for (i, l) in lines[first..].iter().enumerate() {
        let shown: String = l.chars().take(SCREEN_COLS - 6).collect();
        put_at(3 + i, 4, &shown, TEXT);
    }
    put_at(INPUT_ROW - 1, 2, "Your thought:", DIM);
    draw_input(line);
    put_at(24, 2, "Type, then Enter: save   Tab: read back   Backspace: delete   Esc: menu", DIM);
}

fn draw_input(line: &str) {
    vga::clear_row_attr(INPUT_ROW, TEXT);
    let room = SCREEN_COLS - 6;
    let start = line.len().saturating_sub(room);
    put_at(INPUT_ROW, 2, "> ", ACCENT);
    put_at(INPUT_ROW, 4, &line[start..], TEXT);
    put_at(INPUT_ROW, 4 + line.len() - start, "_", ACCENT);
}

fn save(text: &str) -> usize {
    let (h, m) = timer::rtc_hhmm();
    let mut data = fs::read(FILE).unwrap_or_default();
    if !data.is_empty() && !data.ends_with('\n') {
        data.push('\n');
    }
    data.push_str(&format!("[{:02}:{:02}] {}\n", h, m, text));
    fs::write(FILE, &data);
    count()
}

fn read_back() {
    let saved = fs::read(FILE).unwrap_or_default();
    let lines: Vec<&str> = saved.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        say("No saved thoughts yet.");
        return;
    }
    serial::write_str(&format!("Your saved thoughts, {} in all:\n", lines.len()));
    for (i, l) in lines.iter().enumerate() {
        serial::write_str(&format!("{}. {}\n", i + 1, l));
    }
    say(&format!("Read back {} thoughts on the serial console. Keep typing, or Esc for the menu.", lines.len()));
}

pub fn run() {
    let mut line = String::new();
    draw(&line);
    let n = count();
    serial::write_str("\n[Thoughts] 4am Thoughts notepad.\n");
    serial::write_str(&format!(
        "[Thoughts] {} saved in thoughts.txt. Type a thought and press Enter to save it. Tab reads them back. Esc goes back to the menu.\n",
        if n == 1 { String::from("1 thought") } else { format!("{} thoughts", n) }
    ));
    say("Type a thought, then press Enter to save it.");
    loop {
        let b = input::read_key();
        match b {
            b'\n' => {
                serial::write_str("\n");
                let text = line.trim();
                if text.is_empty() {
                    say("Nothing to save yet. Type a thought first.");
                } else {
                    let total = save(text);
                    line.clear();
                    draw(&line);
                    say(&format!("Saved thought {} to thoughts.txt.", total));
                }
                draw_input(&line);
            }
            b'\t' => read_back(),
            0x1b => break,
            0x08 => {
                if line.pop().is_some() {
                    serial::write_str("\x08");
                    draw_input(&line);
                }
            }
            0x20..=0x7e => {
                if line.len() < MAX {
                    line.push(b as char);
                    let mut c = [0u8; 1];
                    c[0] = b;
                    serial::write_str(core::str::from_utf8(&c).unwrap_or("?"));
                    draw_input(&line);
                } else {
                    say("That thought is full (200 characters). Press Enter to save it.");
                }
            }
            _ => {}
        }
    }
}
