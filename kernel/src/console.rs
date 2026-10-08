//! The console: one `print!` that writes to the VGA screen AND the serial port.
//! Screen and serial always carry the same text (accessibility: a screen
//! reader on the serial console hears everything that is shown).

use crate::{serial, vga};
use core::fmt::{self, Write};

struct Mirror;

impl Write for Mirror {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        vga::WRITER.lock().write_str(s)?;
        serial::write_str(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    crate::cpu::interrupts::without_interrupts(|| {
        let _ = Mirror.write_fmt(args);
    });
}

pub fn colored(color: vga::Color, args: fmt::Arguments) {
    crate::cpu::interrupts::without_interrupts(|| {
        vga::WRITER.lock().set_color(color);
        let _ = Mirror.write_fmt(args);
        vga::WRITER.lock().reset_color();
    });
}

pub fn clear() {
    crate::cpu::interrupts::without_interrupts(|| {
        vga::WRITER.lock().clear();
        // No ANSI escapes on serial: a screen reader would read them aloud.
        serial::write_str("\n[screen cleared]\n");
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Boot log line: "[ ok ] message" with a green tag on screen.
#[macro_export]
macro_rules! ok {
    ($($arg:tt)*) => {{
        $crate::console::colored($crate::vga::Color::LightGreen, format_args!("[ ok ] "));
        $crate::println!($($arg)*);
    }};
}
