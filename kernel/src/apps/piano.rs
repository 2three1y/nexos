//! Piano: a keyboard piano on the PC speaker. Keys act at once (no Enter).
//! 1 to 8 play a C major scale, + and - change octave, s plays a song,
//! r replays your last notes. Every note is printed by name, so a screen
//! reader says "E4" as it sounds.

use super::ui;
use crate::{input, println, timer};
use alloc::vec::Vec;

const NAMES: [&str; 8] = ["C", "D", "E", "F", "G", "A", "B", "C"];
/// Octave 4 frequencies in Hz; the last C is octave 5.
const FREQS: [u32; 8] = [262, 294, 330, 349, 392, 440, 494, 523];

fn freq(i: usize, octave: i32) -> u32 {
    let f = FREQS[i];
    match octave {
        o if o > 4 => f << (o - 4),
        o if o < 4 => f >> (4 - o),
        _ => f,
    }
}

fn play(i: usize, octave: i32, ms: u64) {
    let oct = if i == 7 { octave + 1 } else { octave };
    println!("{}{}", NAMES[i], oct);
    timer::tone(freq(i, octave), ms);
}

/// Ode to Joy, as scale degrees (0 = C).
const SONG: &[usize] = &[2, 2, 3, 4, 4, 3, 2, 1, 0, 0, 1, 2, 2, 1, 1];

fn help() {
    ui::heading("Piano help");
    println!("1 to 8 play C D E F G A B C.");
    println!("+ goes up an octave, - goes down.");
    println!("s plays Ode to Joy. r replays your last notes.");
    println!("q quits.");
}

pub fn run() {
    ui::title("Piano", "1.0", "Keys 1 to 8 play notes. No need to press Enter.");
    let mut octave = 4;
    let mut recent: Vec<(usize, i32)> = Vec::new();
    loop {
        let k = input::read_key();
        match k {
            b'1'..=b'8' => {
                let i = (k - b'1') as usize;
                play(i, octave, 260);
                recent.push((i, octave));
                if recent.len() > 16 {
                    recent.remove(0);
                }
            }
            b'+' | b'=' => {
                if octave < 6 { octave += 1; }
                println!("Octave {}.", octave);
            }
            b'-' | b'_' => {
                if octave > 3 { octave -= 1; }
                println!("Octave {}.", octave);
            }
            b's' => {
                println!("Playing Ode to Joy.");
                for (n, &d) in SONG.iter().enumerate() {
                    play(d, octave, if n == SONG.len() - 1 { 520 } else { 260 });
                    timer::sleep_ms(30);
                }
                println!("Song done.");
            }
            b'r' => {
                if recent.is_empty() {
                    println!("Nothing to replay yet. Play some notes first.");
                } else {
                    println!("Replaying {}.", ui::plural(recent.len(), "note", "notes"));
                    for &(i, o) in recent.clone().iter() {
                        play(i, o, 260);
                        timer::sleep_ms(30);
                    }
                }
            }
            b'h' | b'?' => help(),
            b'q' | ui::ESC => break,
            b'\n' | b' ' => {}
            _ => println!("That key has no note. Use 1 to 8, or h for help."),
        }
    }
    ui::closed("Piano");
}
