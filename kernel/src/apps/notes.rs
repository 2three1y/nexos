//! Notes: write, read and delete short notes. Each note is a file in the
//! in-memory filesystem named `notes/<title>`, so `ls` and `cat` see it too.

use super::ui;
use crate::{fs, println, timer};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

const PREFIX: &str = "notes/";

fn notes() -> Vec<(String, usize)> {
    fs::list().into_iter().filter(|(n, _)| n.starts_with(PREFIX)).collect()
}

fn clean_title(t: &str) -> String {
    let s: String = t
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c.to_ascii_lowercase() } else { '-' })
        .take(24)
        .collect();
    let s = String::from(s.trim_matches('-'));
    if s.is_empty() { format!("note-{}", notes().len() + 1) } else { s }
}

fn list() {
    let all = notes();
    if all.is_empty() {
        println!("You have no notes yet. Type n to write one.");
        return;
    }
    ui::heading(&format!("You have {}:", ui::plural(all.len(), "note", "notes")));
    for (i, (name, size)) in all.iter().enumerate() {
        println!("{}. {}, {}.", i + 1, &name[PREFIX.len()..], ui::plural(*size, "character", "characters"));
    }
}

fn pick(arg: &str) -> Option<String> {
    let all = notes();
    if let Ok(n) = arg.parse::<usize>() {
        return all.get(n.wrapping_sub(1)).map(|(name, _)| name.clone());
    }
    let want = format!("{}{}", PREFIX, clean_title(arg));
    all.into_iter().map(|(n, _)| n).find(|n| *n == want)
}

fn new_note() {
    let Some(title) = ui::read_line("Title: ") else { return };
    let Some(text) = ui::read_line("Note: ") else {
        println!("Not saved.");
        return;
    };
    if text.is_empty() {
        println!("The note is empty, so nothing was saved.");
        return;
    }
    let name = format!("{}{}", PREFIX, clean_title(&title));
    let replaced = fs::read(&name).is_some();
    fs::write(&name, &text);
    let n = notes().iter().position(|(x, _)| *x == name).map(|i| i + 1).unwrap_or(0);
    ui::done(format_args!("{} note {}, {}.", if replaced { "Replaced" } else { "Saved" }, n, &name[PREFIX.len()..]));
    timer::ok_click();
}

fn read(arg: &str) {
    match pick(arg) {
        Some(name) => {
            println!("{}:", &name[PREFIX.len()..]);
            println!("{}", fs::read(&name).unwrap_or_default());
        }
        None => ui::error(format_args!("no note \"{}\". Type l to list your notes.", arg)),
    }
}

fn delete(arg: &str) {
    match pick(arg) {
        Some(name) => {
            let title = &name[PREFIX.len()..];
            if ui::confirm(&format!("Delete {}?", title)) {
                fs::remove(&name);
                ui::done(format_args!("Deleted {}.", title));
            } else {
                println!("Kept {}.", title);
            }
        }
        None => ui::error(format_args!("no note \"{}\". Type l to list your notes.", arg)),
    }
}

fn help() {
    ui::heading("Notes help");
    println!("n: write a new note.");
    println!("l: list your notes.");
    println!("r 2: read note 2. A title works too.");
    println!("d 2: delete note 2.");
    println!("q: quit. Notes are files, so cat notes/<title> works in the shell.");
}

pub fn run() {
    ui::title("Notes", "1.0", &format!("You have {}.", ui::plural(notes().len(), "note", "notes")));
    loop {
        let Some(line) = ui::read_line("notes> ") else { break };
        let (cmd, arg) = line.split_once(' ').map(|(c, a)| (c, a.trim())).unwrap_or((line.as_str(), ""));
        match cmd {
            "" => {}
            c if ui::is_quit(c) => break,
            c if ui::is_help(c) => help(),
            "n" | "new" | "w" | "write" => new_note(),
            "l" | "list" | "ls" => list(),
            "r" | "read" | "open" => read(arg),
            "d" | "delete" | "del" | "rm" => delete(arg),
            c if c.parse::<usize>().is_ok() => read(c),
            other => ui::error(format_args!("unknown command \"{}\". Type h for help.", other)),
        }
    }
    ui::closed("Notes");
}
