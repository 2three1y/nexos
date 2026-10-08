//! Keyboard-only input. PS/2 scancodes (IRQ1) are decoded with a US layout;
//! bytes arriving on the serial line (IRQ4) are accepted too, so the whole
//! OS can be driven from a serial terminal / screen reader with no screen.

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

const CAP: usize = 256;

struct Ring {
    buf: [u8; CAP],
    head: usize,
    tail: usize,
}

static QUEUE: Mutex<Ring> = Mutex::new(Ring { buf: [0; CAP], head: 0, tail: 0 });
static KEYBOARD: Mutex<Option<Keyboard<layouts::Us104Key, ScancodeSet1>>> = Mutex::new(None);

fn push(b: u8) {
    let mut q = QUEUE.lock();
    let next = (q.head + 1) % CAP;
    if next != q.tail {
        let h = q.head;
        q.buf[h] = b;
        q.head = next;
    }
}

pub fn push_scancode(code: u8) {
    let mut kb = KEYBOARD.lock();
    let kb = kb.get_or_insert_with(|| {
        Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore)
    });
    if let Ok(Some(event)) = kb.add_byte(code) {
        if let Some(DecodedKey::Unicode(c)) = kb.process_keyevent(event) {
            if c.is_ascii() {
                push(c as u8);
            }
        }
    }
}

pub fn push_serial(b: u8) {
    match b {
        b'\r' => push(b'\n'),
        b'\n' => {}
        0x7f => push(0x08),
        _ => push(b),
    }
}

/// Pop one input byte. Call with interrupts disabled or tolerate races.
pub fn pop() -> Option<u8> {
    let mut q = QUEUE.lock();
    if q.head == q.tail {
        None
    } else {
        let b = q.buf[q.tail];
        q.tail = (q.tail + 1) % CAP;
        Some(b)
    }
}

/// Block (sleeping the CPU with `hlt`) until a key arrives.
pub fn read_key() -> u8 {
    use x86_64::instructions::interrupts;
    loop {
        interrupts::disable();
        if let Some(b) = pop() {
            interrupts::enable();
            return b;
        }
        interrupts::enable_and_hlt();
    }
}
