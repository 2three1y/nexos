//! A tiny in-memory filesystem (flat namespace, lives on the kernel heap).
//! `ls`, `cat`, `write` and `rm` in the shell operate on it.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;

pub struct File {
    pub name: String,
    pub data: String,
}

static FILES: Mutex<Vec<File>> = Mutex::new(Vec::new());

pub fn init() -> usize {
    let mut fs = FILES.lock();
    fs.push(File {
        name: "readme.txt".into(),
        data: "Welcome to Looscid OS, running on the NexOS kernel.\n\
               Everything works from the keyboard, and everything on screen\n\
               is mirrored to the serial port for screen readers.\n\
               Type 'help' to see what you can do."
            .into(),
    });
    fs.push(File {
        name: "roadmap.txt".into(),
        data: "Done: boot, GDT/IDT, timer, keyboard, heap, paging, syscalls,\n\
               a ring-3 program, this filesystem.\n\
               Next: processes + scheduler, ELF apps on disk, then Looscid's\n\
               local-first workspace running as the first real app."
            .into(),
    });
    fs.push(File { name: "motd.txt".into(), data: "Be kind. Ship small. Keep it accessible.".into() });
    fs.len()
}

pub fn list() -> Vec<(String, usize)> {
    FILES.lock().iter().map(|f| (f.name.clone(), f.data.len())).collect()
}

pub fn read(name: &str) -> Option<String> {
    FILES.lock().iter().find(|f| f.name == name).map(|f| f.data.clone())
}

pub fn write(name: &str, data: &str) {
    let mut fs = FILES.lock();
    if let Some(f) = fs.iter_mut().find(|f| f.name == name) {
        f.data = data.to_string();
    } else {
        fs.push(File { name: name.to_string(), data: data.to_string() });
    }
}

pub fn remove(name: &str) -> bool {
    let mut fs = FILES.lock();
    let before = fs.len();
    fs.retain(|f| f.name != name);
    fs.len() != before
}
