//! Clock: the time, a countdown timer, a stopwatch with laps, and an alarm.
//! Uses the PIT (100 Hz) for counting and the CMOS clock for the time of day.
//! Countdowns say each step in words; the finish rings the alarm chime.

use super::ui;
use crate::{input, println, timer};
use alloc::format;

/// "30", "30s", "5m", "1m30s", "2m 10s" -> seconds.
fn parse_duration(s: &str) -> Option<u64> {
    let s: alloc::string::String = s.chars().filter(|c| *c != ' ').collect();
    if s.is_empty() { return None; }
    let (mut total, mut num) = (0u64, 0u64);
    let mut have = false;
    for c in s.chars() {
        match c {
            '0'..='9' => { num = num.checked_mul(10)? + c as u64 - '0' as u64; have = true; }
            'h' => { total += num * 3600; num = 0; have = false; }
            'm' => { total += num * 60; num = 0; have = false; }
            's' => { total += num; num = 0; have = false; }
            _ => return None,
        }
    }
    if have { total += num; }
    (total > 0 && total <= 24 * 3600).then_some(total)
}

/// Wait until `until_ms`, or a key. Returns false if a key cancelled it.
fn wait_until(until_ms: u64) -> bool {
    while timer::uptime_ms() < until_ms {
        if input::try_key().is_some() {
            return false;
        }
        crate::cpu::hlt();
    }
    true
}

fn countdown(secs: u64) {
    println!("Timer set for {}. Press any key to cancel.", ui::duration_text(secs));
    let start = timer::uptime_ms();
    for left in (1..=secs).rev() {
        let say = left <= 5 || left == 10 || left == 30 || (left % 60 == 0);
        if say && left != secs {
            println!("{} left.", ui::duration_text(left));
        }
        if !wait_until(start + (secs - left + 1) * 1000) {
            println!("Timer cancelled.");
            return;
        }
    }
    ui::done(format_args!("Time is up! {} have passed.", ui::duration_text(secs)));
    timer::alarm_chime();
}

fn secs_text(ms: u64) -> alloc::string::String {
    format!("{}.{:02} seconds", ms / 1000, (ms % 1000) / 10)
}

fn stopwatch() {
    println!("Stopwatch started. Enter marks a lap. Any other key stops it.");
    let start = timer::uptime_ms();
    let mut laps = 0;
    loop {
        if let Some(k) = input::try_key() {
            let t = timer::uptime_ms() - start;
            if k == b'\n' {
                laps += 1;
                println!("Lap {}: {}.", laps, secs_text(t));
            } else {
                ui::done(format_args!("Stopped at {}.", secs_text(t)));
                timer::ok_click();
                return;
            }
        }
        crate::cpu::hlt();
    }
}

fn alarm(arg: &str) {
    let parsed = arg.split_once(':').and_then(|(h, m)| Some((h.trim().parse::<u8>().ok()?, m.trim().parse::<u8>().ok()?)));
    let Some((h, m)) = parsed.filter(|(h, m)| *h < 24 && *m < 60) else {
        ui::error(format_args!("give the alarm time in 24 hour form, like alarm 7:30 or alarm 21:05."));
        return;
    };
    println!("Alarm set for {}:{:02}. It is {} now. Press any key to cancel.", h, m, ui::clock_text());
    loop {
        if timer::rtc_hhmm() == (h, m) {
            ui::done(format_args!("Alarm! It is {}.", ui::clock_text()));
            timer::alarm_chime();
            return;
        }
        if !wait_until(timer::uptime_ms() + 1000) {
            println!("Alarm cancelled.");
            return;
        }
    }
}

fn help() {
    ui::heading("Clock help");
    println!("t: say the time.");
    println!("timer 30: count down 30 seconds. timer 5m or timer 1m30s work too.");
    println!("sw: stopwatch. Enter marks a lap, any other key stops.");
    println!("alarm 7:30: ring at 7:30, 24 hour time.");
    println!("q: quit.");
}

pub fn run() {
    ui::title("Clock", "1.0", &format!("It is {}.", ui::clock_text()));
    loop {
        let Some(line) = ui::read_line("clock> ") else { break };
        let (cmd, arg) = line.split_once(' ').map(|(c, a)| (c, a.trim())).unwrap_or((line.as_str(), ""));
        match cmd {
            "" => {}
            c if ui::is_quit(c) => break,
            c if ui::is_help(c) => help(),
            "t" | "time" | "now" => println!("It is {}. Up for {}.", ui::clock_text(), ui::duration_text(timer::uptime_ms() / 1000)),
            "timer" | "countdown" => match parse_duration(arg) {
                Some(s) => countdown(s),
                None => ui::error(format_args!("how long? For example timer 30 or timer 5m.")),
            },
            "sw" | "stopwatch" => stopwatch(),
            "alarm" => alarm(arg),
            other => ui::error(format_args!("unknown command \"{}\". Type h for help.", other)),
        }
    }
    ui::closed("Clock");
}
