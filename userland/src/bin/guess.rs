//! Guess the Number — a ring-3 NexOS game, available from the App Store.
//!
//! Built on the `nexos` app API: everything goes through system calls.
//! Higher and lower are said in words and also played as a rising or
//! falling tone, so it works by ear.

#![no_std]
#![no_main]

use nexos::{beep, println, read_line, sleep_ms, title, uptime_ms, Buf, Rng};

const MAX: u64 = 100;
const TRIES: u32 = 7;

fn win_tune() {
    for (f, ms) in [(523, 90), (659, 90), (784, 90), (1047, 260)] {
        beep(f, ms);
        sleep_ms(20);
    }
}

fn help() {
    println!("Guess the Number help");
    println!("I pick a number from 1 to {}. You have {} guesses.", MAX, TRIES);
    println!("After each guess I say higher or lower.");
    println!("A rising tone means go higher, a falling tone means go lower.");
    println!("Type a number and press Enter. q quits.");
}

/// One round. Returns false if the player quit.
fn round(rng: &mut Rng) -> bool {
    rng.reseed(uptime_ms());
    let secret = rng.range(1, MAX);
    println!("I'm thinking of a number from 1 to {}. You have {} guesses.", MAX, TRIES);
    let mut line: Buf<16> = Buf::new();
    let mut tries = 0;
    while tries < TRIES {
        let mut prompt: Buf<32> = Buf::new();
        let _ = core::fmt::Write::write_fmt(&mut prompt, format_args!("Guess {} of {}: ", tries + 1, TRIES));
        if !read_line(prompt.as_str(), &mut line) {
            return false;
        }
        let text = line.as_str().trim();
        match text {
            "" => continue,
            "q" | "quit" | "exit" => return false,
            "h" | "help" | "?" => {
                help();
                continue;
            }
            _ => {}
        }
        let Ok(g) = text.parse::<u64>() else {
            println!("Error: that is not a number. Type a number from 1 to {}.", MAX);
            beep(196, 160);
            continue;
        };
        if !(1..=MAX).contains(&g) {
            println!("Error: pick a number from 1 to {}.", MAX);
            beep(196, 160);
            continue;
        }
        tries += 1;
        if g == secret {
            println!("Correct! It was {}. You got it in {} {}.", secret, tries, if tries == 1 { "guess" } else { "guesses" });
            win_tune();
            return true;
        }
        let left = TRIES - tries;
        if g < secret {
            println!("Higher than {}. {} left.", g, left);
            beep(440, 90);
            beep(660, 120);
        } else {
            println!("Lower than {}. {} left.", g, left);
            beep(660, 90);
            beep(440, 120);
        }
    }
    println!("Out of guesses. The number was {}.", secret);
    beep(330, 150);
    beep(262, 300);
    true
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    title("Guess the Number", "1.0", "Find my secret number by ear or by eye.");
    let mut rng = Rng::seeded();
    loop {
        if !round(&mut rng) {
            break;
        }
        let mut ans: Buf<8> = Buf::new();
        if !read_line("Play again? y or n: ", &mut ans) || !matches!(ans.as_str().trim(), "y" | "yes") {
            break;
        }
    }
    println!("Closed Guess the Number.");
    nexos::exit(0)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    nexos::write("guess: panic\n");
    nexos::exit(1)
}
