//! VGA text-mode console (0xB8000, 80x25) with scrolling and a hardware cursor.
//!
//! High contrast by default: bright white on black. `theme light` flips to
//! black on bright white for people who read better with a light background.

use core::fmt;
use spin::Mutex;
use x86_64::instructions::port::Port;

const COLS: usize = 80;
const ROWS: usize = 25;
const VGA_MEM: usize = 0xB8000;

#[derive(Clone, Copy)]
#[repr(u8)]
#[allow(dead_code)]
pub enum Color {
    Black = 0, Blue = 1, Cyan = 3, LightGrey = 7, LightCyan = 11,
    LightGreen = 10, LightRed = 12, Yellow = 14, White = 15,
}

pub struct Writer {
    col: usize,
    row: usize,
    attr: u8,
    base: u8,
}

pub static WRITER: Mutex<Writer> = Mutex::new(Writer { col: 0, row: 0, attr: 0x0F, base: 0x0F });

impl Writer {
    fn cell(&self, row: usize, col: usize) -> *mut u16 {
        (VGA_MEM + (row * COLS + col) * 2) as *mut u16
    }

    pub fn clear(&mut self) {
        for r in 0..ROWS {
            self.clear_row(r);
        }
        self.row = 0;
        self.col = 0;
        self.update_cursor();
    }

    fn clear_row(&mut self, r: usize) {
        let blank = (self.base as u16) << 8 | b' ' as u16;
        for c in 0..COLS {
            unsafe { self.cell(r, c).write_volatile(blank) };
        }
    }

    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 < ROWS {
            self.row += 1;
        } else {
            for r in 1..ROWS {
                for c in 0..COLS {
                    unsafe {
                        let v = self.cell(r, c).read_volatile();
                        self.cell(r - 1, c).write_volatile(v);
                    }
                }
            }
            self.clear_row(ROWS - 1);
        }
    }

    pub fn put_byte(&mut self, b: u8) {
        match b {
            b'\n' => self.newline(),
            0x08 => {
                if self.col > 0 {
                    self.col -= 1;
                    let blank = (self.base as u16) << 8 | b' ' as u16;
                    unsafe { self.cell(self.row, self.col).write_volatile(blank) };
                }
            }
            b => {
                if self.col >= COLS {
                    self.newline();
                }
                let b = if (0x20..0x7f).contains(&b) { b } else { b'?' };
                unsafe { self.cell(self.row, self.col).write_volatile((self.attr as u16) << 8 | b as u16) };
                self.col += 1;
            }
        }
    }

    pub fn set_color(&mut self, fg: Color) {
        let bg = self.base >> 4;
        // In the light theme every foreground is drawn black for contrast.
        self.attr = if bg == 0 { (fg as u8) | (bg << 4) } else { self.base };
    }

    pub fn reset_color(&mut self) {
        self.attr = self.base;
    }

    pub fn set_theme(&mut self, light: bool) {
        self.base = if light { 0xF0 } else { 0x0F };
        self.attr = self.base;
        // Repaint existing cells with the new base attribute.
        for r in 0..ROWS {
            for c in 0..COLS {
                unsafe {
                    let v = self.cell(r, c).read_volatile();
                    self.cell(r, c).write_volatile((self.base as u16) << 8 | (v & 0xFF));
                }
            }
        }
    }

    pub fn update_cursor(&self) {
        let pos = (self.row * COLS + self.col.min(COLS - 1)) as u16;
        let mut idx: Port<u8> = Port::new(0x3D4);
        let mut dat: Port<u8> = Port::new(0x3D5);
        unsafe {
            idx.write(0x0F);
            dat.write((pos & 0xFF) as u8);
            idx.write(0x0E);
            dat.write((pos >> 8) as u8);
        }
    }
}

/// Direct cell drawing for full-screen apps (no cursor movement).
pub fn put_at(row: usize, col: usize, s: &str, attr: u8) {
    for (i, b) in s.bytes().enumerate() {
        let c = col + i;
        if row >= ROWS || c >= COLS {
            break;
        }
        let b = if (0x20..0x7f).contains(&b) { b } else { b'?' };
        unsafe { ((VGA_MEM + (row * COLS + c) * 2) as *mut u16).write_volatile((attr as u16) << 8 | b as u16) };
    }
}

/// Fill a whole row with spaces in the given attribute.
pub fn clear_row_attr(row: usize, attr: u8) {
    for c in 0..COLS {
        unsafe { ((VGA_MEM + (row * COLS + c) * 2) as *mut u16).write_volatile((attr as u16) << 8 | b' ' as u16) };
    }
}

pub fn hide_cursor() {
    let mut idx: Port<u8> = Port::new(0x3D4);
    let mut dat: Port<u8> = Port::new(0x3D5);
    unsafe {
        idx.write(0x0A);
        dat.write(0x20);
    }
}

pub fn show_cursor() {
    let mut idx: Port<u8> = Port::new(0x3D4);
    let mut dat: Port<u8> = Port::new(0x3D5);
    unsafe {
        idx.write(0x0A);
        dat.write(0x0D);
        idx.write(0x0B);
        dat.write(0x0F);
    }
}

pub const SCREEN_ROWS: usize = ROWS;
pub const SCREEN_COLS: usize = COLS;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.put_byte(b);
        }
        self.update_cursor();
        Ok(())
    }
}
