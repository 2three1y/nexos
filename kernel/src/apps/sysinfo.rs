//! System Info: a plain-words report on this computer.

use super::{store, ui};
use crate::{allocator, fs, input, memory, println, timer};
use core::sync::atomic::Ordering;

fn report() {
    ui::heading("System Info");
    println!("NexOS {}, x86_64.", env!("CARGO_PKG_VERSION"));
    println!("Uptime: {}.", ui::duration_text(timer::uptime_ms() / 1000));
    println!("Clock: {}.", ui::clock_text());
    let (used, free) = allocator::stats();
    let total = used + free;
    println!(
        "Memory: {} KB of the {} KB heap used, {} percent.",
        used / 1024,
        total / 1024,
        if total > 0 { used * 100 / total } else { 0 }
    );
    let (fu, ft) = memory::frame_stats();
    println!("RAM: {} MB usable. {} of {} page frames handed out.", ft * 4 / 1024, fu, ft);
    let cat = store::catalog();
    let inst = store::installed_list().len();
    println!("Apps: {} installed, {} in the App Store.", inst, cat.len());
    println!("Files: {}.", fs::list().len());
    println!(
        "Sound: {}. {} played since boot. Soundscape: {}.",
        if timer::MUTED.load(Ordering::Relaxed) { "muted" } else { "on" },
        ui::plural(timer::NOTES.load(Ordering::Relaxed) as usize, "note", "notes"),
        crate::apps::soundscape::playing()
    );
}

pub fn run() {
    ui::title("System Info", "1.0", "Press r to refresh.");
    report();
    loop {
        match input::read_key() {
            b'r' => report(),
            b'h' | b'?' => println!("r refreshes the report. q quits."),
            b'q' | ui::ESC => break,
            _ => {}
        }
    }
    ui::closed("System Info");
}
