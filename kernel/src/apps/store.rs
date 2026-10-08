//! The Looscid App Store.
//!
//! Every app ships as a package manifest (a small `key = value` text file, see
//! docs/APPS.md). The catalog is bundled into the OS image from
//! kernel/catalog/*.app and unpacked into the filesystem at boot as
//! `store/<id>.app`, so it works offline (there is no network stack yet).
//! Which apps are installed lives in `system/installed.txt` (one
//! `id version` per line) and lasts for the session.
//!
//! The UI is numbered, keyboard-only plain text that reads well on a serial
//! screen reader: list, info, install, uninstall, update, search, installed.

use super::ui;
use crate::vga::Color;
use crate::{console, fs, println, timer, user};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub const INSTALLED_FILE: &str = "system/installed.txt";
const CATALOG_PREFIX: &str = "store/";

/// Package manifests bundled into the image (the offline catalog).
static BUNDLED: &[(&str, &str)] = &[
    ("store", include_str!("../../catalog/store.app")),
    ("notes", include_str!("../../catalog/notes.app")),
    ("calc", include_str!("../../catalog/calc.app")),
    ("clock", include_str!("../../catalog/clock.app")),
    ("sysinfo", include_str!("../../catalog/sysinfo.app")),
    ("insomnia", include_str!("../../catalog/insomnia.app")),
    ("hello", include_str!("../../catalog/hello.app")),
    ("piano", include_str!("../../catalog/piano.app")),
    ("guess", include_str!("../../catalog/guess.app")),
];

#[derive(Clone)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub size_kb: u32,
    pub kind: String,
    pub entry: String,
    pub preinstalled: bool,
    pub removable: bool,
    pub aliases: Vec<String>,
    pub summary: String,
    pub description: String,
    pub keys: String,
}

/// Parse a manifest. Unknown keys are ignored so newer manifests still load.
pub fn parse(text: &str) -> Option<Manifest> {
    let mut m = Manifest {
        id: String::new(), name: String::new(), version: "1.0".into(), category: "Other".into(),
        size_kb: 0, kind: "native".into(), entry: String::new(), preinstalled: false, removable: true,
        aliases: Vec::new(), summary: String::new(), description: String::new(), keys: String::new(),
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "id" => m.id = v.into(),
            "name" => m.name = v.into(),
            "version" => m.version = v.into(),
            "category" => m.category = v.into(),
            "size_kb" => m.size_kb = v.parse().unwrap_or(0),
            "kind" => m.kind = v.into(),
            "entry" => m.entry = v.into(),
            "preinstalled" => m.preinstalled = v == "yes",
            "removable" => m.removable = v != "no",
            "aliases" => m.aliases = v.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect(),
            "summary" => m.summary = v.into(),
            "description" => m.description = v.into(),
            "keys" => m.keys = v.into(),
            _ => {}
        }
    }
    if m.id.is_empty() || m.name.is_empty() {
        return None;
    }
    if m.entry.is_empty() {
        m.entry = m.id.clone();
    }
    if m.kind == "elf" {
        // Ring-3 apps report the real size of their program image.
        if let Some(bytes) = user::image_size(&m.entry) {
            m.size_kb = ((bytes + 1023) / 1024) as u32;
        }
    }
    Some(m)
}

/// Unpack the bundled catalog into the filesystem and record the
/// preinstalled apps. Returns (apps in catalog, apps installed).
pub fn init() -> (usize, usize) {
    let mut installed = String::new();
    for (id, text) in BUNDLED {
        fs::write(&format!("{}{}.app", CATALOG_PREFIX, id), text);
        if let Some(m) = parse(text) {
            if m.preinstalled {
                installed.push_str(&format!("{} {}\n", m.id, m.version));
            }
        }
    }
    fs::write(INSTALLED_FILE, &installed);
    (catalog().len(), installed_list().len())
}

/// The catalog, read from the `store/*.app` files, in catalog order.
pub fn catalog() -> Vec<Manifest> {
    fs::list()
        .into_iter()
        .filter(|(n, _)| n.starts_with(CATALOG_PREFIX) && n.ends_with(".app"))
        .filter_map(|(n, _)| fs::read(&n).and_then(|t| parse(&t)))
        .collect()
}

/// (id, installed version) pairs.
pub fn installed_list() -> Vec<(String, String)> {
    fs::read(INSTALLED_FILE)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut p = l.split_whitespace();
            Some((p.next()?.to_string(), p.next().unwrap_or("1.0").to_string()))
        })
        .collect()
}

fn installed_version(id: &str) -> Option<String> {
    installed_list().into_iter().find(|(i, _)| i == id).map(|(_, v)| v)
}

pub fn is_installed(id: &str) -> bool {
    installed_version(id).is_some()
}

fn save_installed(list: &[(String, String)]) {
    let mut s = String::new();
    for (id, v) in list {
        s.push_str(&format!("{} {}\n", id, v));
    }
    fs::write(INSTALLED_FILE, &s);
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

/// Find an app by catalog number, id, name or alias (case and spaces ignored).
pub fn find(query: &str) -> Option<(usize, Manifest)> {
    let cat = catalog();
    if let Ok(n) = query.trim().parse::<usize>() {
        return cat.get(n.wrapping_sub(1)).cloned().map(|m| (n, m));
    }
    let q = squash(query);
    if q.is_empty() {
        return None;
    }
    cat.into_iter().enumerate().find_map(|(i, m)| {
        let hit = squash(&m.id) == q || squash(&m.name) == q || m.aliases.iter().any(|a| squash(a) == q);
        hit.then(|| (i + 1, m))
    })
}

fn status_word(m: &Manifest) -> &'static str {
    if is_installed(&m.id) { "Installed" } else { "Not installed" }
}

fn print_entry(n: usize, m: &Manifest) {
    println!("{}. {} {}, {}. {}.", n, m.name, m.version, m.category, status_word(m));
    println!("   {}", m.summary);
}

pub fn list() {
    let cat = catalog();
    let inst = cat.iter().filter(|m| is_installed(&m.id)).count();
    ui::heading(&format!("App Store: {}, {} installed.", ui::plural(cat.len(), "app", "apps"), inst));
    for (i, m) in cat.iter().enumerate() {
        print_entry(i + 1, m);
    }
    println!("Type info and a number for details, or install and a number.");
}

pub fn installed() {
    let cat = catalog();
    let mine: Vec<(usize, &Manifest)> = cat.iter().enumerate().filter(|(_, m)| is_installed(&m.id)).map(|(i, m)| (i + 1, m)).collect();
    ui::heading(&format!("{} installed:", ui::plural(mine.len(), "app", "apps")));
    for (n, m) in mine {
        println!("{}. {} {}. Open it with: {}", n, m.name, installed_version(&m.id).unwrap_or_default(), m.id);
    }
}

pub fn search(word: &str) {
    if word.is_empty() {
        ui::error(format_args!("search needs a word. Example: search music"));
        return;
    }
    let w = word.to_ascii_lowercase();
    let hits: Vec<(usize, Manifest)> = catalog()
        .into_iter()
        .enumerate()
        .filter(|(_, m)| {
            [&m.id, &m.name, &m.category, &m.summary, &m.description].iter().any(|f| f.to_ascii_lowercase().contains(&w))
        })
        .map(|(i, m)| (i + 1, m))
        .collect();
    if hits.is_empty() {
        println!("No apps match \"{}\".", word);
        return;
    }
    ui::heading(&format!("{} for \"{}\":", ui::plural(hits.len(), "match", "matches"), word));
    for (n, m) in &hits {
        print_entry(*n, m);
    }
}

pub fn info(query: &str) {
    let Some((n, m)) = find(query) else { return not_found(query) };
    ui::heading(&format!("{}. {} {}", n, m.name, m.version));
    println!("{}", m.description);
    println!("Category: {}. Size: {} KB.", m.category, m.size_kb);
    println!("Type: {}.", if m.kind == "elf" { "ring 3 program, runs in user mode" } else { "native app, built into the kernel" });
    if !m.keys.is_empty() {
        println!("Keys: {}", m.keys);
    }
    if is_installed(&m.id) {
        let extra = if m.removable { "" } else { " System app, can't be removed." };
        println!("Status: installed. Open it with: {}.{}", m.id, extra);
    } else {
        println!("Status: not installed. To get it, type: install {}", m.id);
    }
}

fn not_found(query: &str) {
    if query.is_empty() {
        ui::error(format_args!("which app? Give a name or a number from the list."));
    } else {
        ui::error(format_args!("no app called \"{}\". Type list to see them all.", query));
    }
}

pub fn install(query: &str) {
    let Some((_, m)) = find(query) else { return not_found(query) };
    if is_installed(&m.id) {
        println!("{} is already installed. Open it with: {}", m.name, m.id);
        return;
    }
    // Verify the package: its program must be present in this image.
    if user::find(&m.entry).is_none() {
        ui::error(format_args!("the {} package is damaged: its program is missing.", m.name));
        return;
    }
    println!("Installing {} {}, {} KB.", m.name, m.version, m.size_kb);
    timer::sleep_ms(250);
    println!("Checking the package. OK.");
    timer::sleep_ms(250);
    let mut list = installed_list();
    list.push((m.id.clone(), m.version.clone()));
    save_installed(&list);
    ui::done(format_args!("Installed {}. Open it with: {}", m.name, m.id));
    timer::install_chime();
}

pub fn uninstall(query: &str, ask: bool) {
    let Some((_, m)) = find(query) else { return not_found(query) };
    if !is_installed(&m.id) {
        ui::error(format_args!("{} is not installed.", m.name));
        return;
    }
    if !m.removable {
        ui::error(format_args!("{} is a system app and can't be removed.", m.name));
        return;
    }
    if ask && !ui::confirm(&format!("Remove {}? Your files stay.", m.name)) {
        println!("Kept {}.", m.name);
        return;
    }
    let list: Vec<(String, String)> = installed_list().into_iter().filter(|(i, _)| *i != m.id).collect();
    save_installed(&list);
    ui::done(format_args!("Removed {}. You can get it again from the store.", m.name));
    timer::uninstall_chime();
}

pub fn update() {
    let cat = catalog();
    let mut list = installed_list();
    let mut updated = 0;
    for (id, ver) in list.iter_mut() {
        if let Some(m) = cat.iter().find(|m| &m.id == id) {
            if &m.version != ver {
                println!("Updated {} from {} to {}.", m.name, ver, m.version);
                *ver = m.version.clone();
                updated += 1;
            }
        }
    }
    if updated == 0 {
        println!("All {} are up to date.", ui::plural(list.len(), "installed app", "installed apps"));
    } else {
        save_installed(&list);
        ui::done(format_args!("{} updated.", ui::plural(updated, "app", "apps")));
        timer::install_chime();
    }
}

/// Open an installed app by id, name, alias or catalog number.
/// Returns false if nothing by that name exists in the catalog.
pub fn launch(query: &str) -> bool {
    let Some((_, m)) = find(query) else { return false };
    if !is_installed(&m.id) {
        ui::error(format_args!("{} is not installed. Get it with: store install {}", m.name, m.id));
        return true;
    }
    match user::find(&m.entry) {
        Some(app) => match user::run(app) {
            Ok(code) if matches!(app.kind, user::AppKind::Elf(_)) && code != 0 => {
                println!("{} ended with code {}.", m.name, code)
            }
            Ok(_) => {}
            Err(e) => ui::error(format_args!("{} could not start: {}", m.name, e)),
        },
        None => ui::error(format_args!("{} is installed but its program is missing.", m.name)),
    }
    true
}

fn help() {
    ui::heading("App Store help");
    println!("list: every app in the store, numbered.");
    println!("info 3: details about app number 3. A name works too.");
    println!("install 3: install an app.");
    println!("uninstall 3: remove an app.");
    println!("update: update installed apps.");
    println!("search music: find apps by word.");
    println!("installed: the apps you have.");
    println!("open 3: open an installed app.");
    println!("A number alone shows that app's info.");
    println!("q quits the store.");
}

/// Run one store command. `interactive` asks before removing. Returns false on quit.
pub fn command(line: &str, interactive: bool) -> bool {
    let (cmd, arg) = match line.split_once(' ') {
        Some((c, a)) => (c, a.trim()),
        None => (line, ""),
    };
    match cmd {
        "" => {}
        c if ui::is_quit(c) => return false,
        c if ui::is_help(c) => help(),
        "list" | "l" | "ls" | "browse" => list(),
        "clear" | "cls" => console::clear(),
        "info" | "i" => info(arg),
        "install" | "get" | "add" => install(arg),
        "uninstall" | "remove" | "rm" | "delete" => uninstall(arg, interactive),
        "update" | "upgrade" => update(),
        "search" | "find" | "s" => search(arg),
        "installed" | "mine" => installed(),
        "open" | "run" => {
            if !launch(arg) {
                not_found(arg);
            }
        }
        n if n.parse::<usize>().is_ok() && arg.is_empty() => info(n),
        other => ui::error(format_args!("unknown store command \"{}\". Type h for help.", other)),
    }
    true
}

/// The interactive store (the `store` app).
pub fn run() {
    let cat = catalog();
    let inst = cat.iter().filter(|m| is_installed(&m.id)).count();
    ui::title("App Store", "1.0", &format!("{} to browse, {} installed.", ui::plural(cat.len(), "app", "apps"), inst));
    println!("Commands: list, info, install, uninstall, update, search, installed.");
    loop {
        let Some(line) = ui::read_line("store> ") else { break };
        if !command(&line, true) {
            break;
        }
    }
    ui::closed("App Store");
}

/// The Home menu: every installed app, numbered, plus the store.
/// The full list is read once; after an app closes, Home says one short line
/// (l reads the list again), so a screen reader isn't flooded.
pub fn home() {
    let mut show_list = true;
    loop {
        let mine: Vec<Manifest> = catalog().into_iter().filter(|m| is_installed(&m.id) && m.id != "store").collect();
        println!();
        if show_list {
            console::colored(Color::LightCyan, format_args!("Home. {} installed.\n", ui::plural(mine.len(), "app", "apps")));
            for (i, m) in mine.iter().enumerate() {
                println!("{}. {}: {}", i + 1, m.name, m.summary);
            }
            println!("s. App Store: get more apps.");
            println!("Type a number to open an app. q goes back to the shell.");
            show_list = false;
        } else {
            println!("Home. Type a number, l to list apps, s for the store, q for the shell.");
        }
        let Some(line) = ui::read_line("home> ") else { break };
        match line.as_str() {
            "" => continue,
            l if ui::is_quit(l) => break,
            "l" | "list" | "ls" => show_list = true,
            "s" | "store" => run(),
            "clear" | "cls" => console::clear(),
            l if ui::is_help(l) => println!("Type the number next to an app and press Enter. l lists the apps. s opens the App Store. q leaves Home."),
            l => match l.parse::<usize>().ok().and_then(|n| mine.get(n.wrapping_sub(1))) {
                Some(m) => {
                    launch(&m.id);
                }
                None => {
                    if !launch(l) {
                        ui::error(format_args!("there is no app number {}. Type l to list them.", l));
                    }
                }
            },
        }
    }
    println!("Left Home. Back at the shell.");
}
