//! COM1 serial port (0x3F8, 115200 8N1).
//!
//! Every character the kernel prints to the screen is mirrored here, so a
//! screen reader attached to a serial console reads exactly what is on screen.
//! Received bytes raise IRQ4 and are fed to the shell like key presses.

use spin::Mutex;
use uart_16550::SerialPort;
use crate::cpu::port::Port;

pub static COM1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

pub fn init() {
    COM1.lock().init(); // also enables the "data received" interrupt
}

/// Write raw text to serial, translating `\n` to `\r\n` for terminals.
pub fn write_str(s: &str) {
    let mut port = COM1.lock();
    for b in s.bytes() {
        match b {
            b'\n' => {
                port.send_raw(b'\r');
                port.send_raw(b'\n');
            }
            0x08 => {
                port.send_raw(0x08);
                port.send_raw(b' ');
                port.send_raw(0x08);
            }
            _ => port.send_raw(b),
        }
    }
}

/// Non-blocking read of one received byte, if any (called from IRQ4).
pub fn try_read() -> Option<u8> {
    let mut lsr: Port<u8> = Port::new(0x3F8 + 5);
    let mut data: Port<u8> = Port::new(0x3F8);
    unsafe {
        if lsr.read() & 1 != 0 {
            Some(data.read())
        } else {
            None
        }
    }
}
