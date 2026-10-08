//! The NexOS shell: a small keyboard-driven command line.

use crate::vga::Color;
use crate::{allocator, console, fs, input, memory, print, println, timer, vga};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

const HELP: &[(&str, &str)] = &[
    ("help", "show this list"),
    ("clear", "clear the screen"),
    ("about", "about NexOS"),
    ("uptime", "time since boot"),
    ("echo <text>", "print text"),
    ("mem", "heap, frame and paging stats"),
    ("ls", "list files"),
    ("cat <file>", "show a file"),
    ("write <file> <text>", "create or replace a file"),
    ("rm <file>", "delete a file"),
    ("home", "your apps, numbered: pick one to open it"),
    ("store", "the App Store (store list, store install piano)"),
    ("apps", "list installed apps"),
    ("<app name>", "open an app: notes, calc, clock, sysinfo, insomnia"),
    ("run <app>", "open an app by name (try: run hello)"),
    ("beep", "play the boot chime"),
    ("mute", "turn sound off/on"),
    ("sound", "speaker status: notes played, soundscape, mute"),
    ("theme dark|light", "high-contrast colour theme"),
    ("int3", "fire a breakpoint exception (handled, then resumes)"),
    ("reboot", "restart the machine"),
];

fn prompt() {
    console::colored(Color::Yellow, format_args!("nexos> "));
}

pub fn run() -> ! {
    println!("NexOS shell ready. Type 'help' for commands.");
    println!("Type home for your apps, or store for the App Store.");
    let mut line = String::new();
    prompt();
    loop {
        let b = input::read_key();
        match b {
            b'\n' => {
                println!();
                execute(line.trim());
                line.clear();
                prompt();
            }
            0x08 => {
                if line.pop().is_some() {
                    print!("\x08");
                }
            }
            0x20..=0x7e => {
                if line.len() < 200 {
                    line.push(b as char);
                    print!("{}", b as char);
                }
            }
            _ => {}
        }
    }
}

fn execute(line: &str) {
    if line.is_empty() {
        return;
    }
    let (cmd, rest) = match line.find(' ') {
        Some(i) => (&line[..i], line[i + 1..].trim()),
        None => (line, ""),
    };
    match cmd {
        "help" => {
            println!("Commands (keyboard only; screen is mirrored to serial):");
            for (c, d) in HELP {
                println!("  {:<20} {}", c, d);
            }
        }
        "clear" => console::clear(),
        "about" => {
            console::colored(Color::LightCyan, format_args!("NexOS v{}\n", env!("CARGO_PKG_VERSION")));
            println!("A hobby operating system in Rust for x86_64, by Hasan (2three1y).");
            println!("NexOS is both the operating system and its kernel.");
            println!("Accessible by design: keyboard-only, high contrast, serial mirror.");
        }
        "uptime" => {
            let ms = timer::uptime_ms();
            let s = ms / 1000;
            println!("up {}h {}m {}.{:02}s ({} timer ticks at {} Hz)", s / 3600, (s / 60) % 60, s % 60, (ms % 1000) / 10, timer::ticks(), timer::HZ);
        }
        "echo" => println!("{}", rest),
        "mem" => {
            let (used, free) = allocator::stats();
            let (fu, ft) = memory::frame_stats();
            println!("heap:   {} bytes used, {} bytes free (at {:#x})", used, free, memory::HEAP_START);
            println!("frames: {} of {} usable 4 KiB frames allocated", fu, ft);
            match memory::translate(memory::HEAP_START) {
                Some(p) => println!("paging: heap {:#x} -> physical {:#x}", memory::HEAP_START, p),
                None => println!("paging: heap not mapped?"),
            }
        }
        "ls" => {
            for (name, size) in fs::list() {
                println!("  {:<20} {} bytes", name, size);
            }
        }
        "cat" => match fs::read(rest) {
            Some(d) => println!("{}", d.trim_end()),
            None => println!("cat: no such file: {}", rest),
        },
        "write" => {
            let mut parts = rest.splitn(2, ' ');
            match (parts.next(), parts.next()) {
                (Some(n), Some(t)) if !n.is_empty() => {
                    fs::write(n, t);
                    println!("wrote {} bytes to {}", t.len(), n);
                }
                _ => println!("usage: write <file> <text>"),
            }
        }
        "rm" => {
            if fs::remove(rest) {
                println!("removed {}", rest);
            } else {
                println!("rm: no such file: {}", rest);
            }
        }
        "apps" => crate::apps::store::installed(),
        "home" => crate::apps::store::home(),
        "store" => {
            if rest.is_empty() {
                crate::apps::store::run();
            } else {
                crate::apps::store::command(rest, false);
            }
        }
        "run" | "open" => {
            let name = if rest.is_empty() { "hello" } else { rest };
            if !crate::apps::store::launch(name) {
                println!("run: no app named '{}'. Type apps for your apps, or store list.", name);
            }
        }
        "beep" => timer::boot_chime(),
        "mute" => {
            let now = !timer::MUTED.load(Ordering::Relaxed);
            timer::MUTED.store(now, Ordering::Relaxed);
            if now {
                timer::speaker_off();
            }
            println!("sound {}", if now { "muted" } else { "on" });
        }
        "sound" => {
            println!(
                "speaker: {} notes programmed on PIT channel 2 since boot; soundscape: {}; sound {}",
                timer::NOTES.load(Ordering::Relaxed),
                crate::apps::soundscape::playing(),
                if timer::MUTED.load(Ordering::Relaxed) { "muted" } else { "on" }
            );
        }
        "theme" => match rest {
            "light" => { vga::WRITER.lock().set_theme(true); println!("theme: black on white"); }
            "dark" => { vga::WRITER.lock().set_theme(false); println!("theme: white on black"); }
            _ => println!("usage: theme dark|light"),
        },
        "int3" => {
            x86_64::instructions::interrupts::int3();
            println!("back in the shell after the breakpoint");
        }
        "reboot" => {
            println!("rebooting...");
            unsafe { x86_64::instructions::port::Port::<u8>::new(0x64).write(0xFE) };
        }
        _ => {
            // An app's name (or alias) opens it, so "notes" or "calculator" just works.
            if line.parse::<usize>().is_ok() || !crate::apps::store::launch(line) {
                let words: Vec<&str> = line.split(' ').collect();
                println!("unknown command: {} (type 'help')", words[0]);
            }
        }
    }
}
