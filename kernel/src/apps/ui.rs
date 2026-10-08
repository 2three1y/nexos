//! The NexOS app toolkit: the small set of helpers every native app uses so
//! they all feel the same. Plain text, short lines, every status said in words,
//! and the same keys everywhere: h for help, q to quit (Esc also quits).
//!
//! Everything printed goes to the screen AND the serial console (see
//! console.rs), so a screen reader on serial hears exactly what is shown.

use crate::vga::Color;
use crate::{console, input, print, println, timer};
use alloc::string::String;

pub const ESC: u8 = 0x1b;

/// Opening lines of an app: its name and version, one sentence, the keys.
pub fn title(name: &str, version: &str, blurb: &str) {
    println!();
    console::colored(Color::LightCyan, format_args!("{} {}\n", name, version));
    println!("{}", blurb);
    println!("Type h for help, q to quit.");
}

/// A section heading, e.g. "Help" or "Your notes".
pub fn heading(text: &str) {
    console::colored(Color::LightCyan, format_args!("{}\n", text));
}

/// Read one line with echo. Enter finishes it; Esc returns None (quit/back).
pub fn read_line(prompt: &str) -> Option<String> {
    console::colored(Color::Yellow, format_args!("{}", prompt));
    let mut line = String::new();
    loop {
        match input::read_key() {
            b'\n' => {
                println!();
                return Some(String::from(line.trim()));
            }
            ESC => {
                println!();
                return None;
            }
            0x08 => {
                if line.pop().is_some() {
                    print!("\x08");
                }
            }
            b @ 0x20..=0x7e => {
                if line.len() < 200 {
                    line.push(b as char);
                    print!("{}", b as char);
                }
            }
            _ => {}
        }
    }
}

/// Ask a yes/no question; Enter alone or anything but y means no.
pub fn confirm(question: &str) -> bool {
    matches!(read_line(&alloc::format!("{} y or n: ", question)).as_deref(), Some("y") | Some("yes"))
}

pub fn is_quit(s: &str) -> bool {
    matches!(s, "q" | "quit" | "exit")
}

pub fn is_help(s: &str) -> bool {
    matches!(s, "h" | "help" | "?")
}

/// Success message in green with a small click.
pub fn done(msg: core::fmt::Arguments) {
    console::colored(Color::LightGreen, format_args!("{}\n", msg));
}

/// Error message: always starts with the word "Error", plus the error tone.
pub fn error(msg: core::fmt::Arguments) {
    console::colored(Color::LightRed, format_args!("Error: {}\n", msg));
    timer::error_tone();
}

pub fn closed(name: &str) {
    println!("Closed {}.", name);
}

/// "1 note" / "3 notes".
pub fn plural(n: usize, one: &str, many: &str) -> String {
    alloc::format!("{} {}", n, if n == 1 { one } else { many })
}

/// Time of day from the real-time clock as words people say: "5:46 AM".
pub fn clock_text() -> String {
    let (h, m) = timer::rtc_hhmm();
    let (h12, ampm) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    alloc::format!("{}:{:02} {}", h12, m, ampm)
}

/// A duration in words: "1 hour, 2 minutes, 5 seconds" / "12 seconds".
pub fn duration_text(total_secs: u64) -> String {
    let (h, m, s) = (total_secs / 3600, (total_secs / 60) % 60, total_secs % 60);
    let mut parts = alloc::vec::Vec::new();
    if h > 0 { parts.push(plural(h as usize, "hour", "hours")); }
    if m > 0 { parts.push(plural(m as usize, "minute", "minutes")); }
    if s > 0 || parts.is_empty() { parts.push(plural(s as usize, "second", "seconds")); }
    parts.join(", ")
}
