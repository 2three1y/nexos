//! IDT, CPU exception handlers, the 8259 PIC, and hardware IRQs (i686 build).
//!
//! Same vector layout as the x86_64 build: 32 = PIT timer, 33 = PS/2
//! keyboard, 36 = COM1 serial, 0x80 = the system-call gate (callable from
//! ring 3, see i686/syscall.rs). 32-bit IDT entries are 8 bytes.

use crate::cpu::port::Port;
use crate::{input, println, serial, timer};
use core::arch::asm;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Irq {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Serial = PIC_1_OFFSET + 4,
}

/// What the CPU pushes for an interrupt (ESP/SS follow only on a ring change).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub eip: usize,
    pub cs: usize,
    pub eflags: usize,
}

static mut IDT: [u64; 256] = [0; 256];

fn gate(handler: usize, dpl: u8) -> u64 {
    let h = handler as u64;
    let type_attr: u64 = 0x8E | ((dpl as u64 & 3) << 5); // present, 32-bit interrupt gate
    (h & 0xFFFF) | (0x08u64 << 16) | (type_attr << 40) | (((h >> 16) & 0xFFFF) << 48)
}

#[repr(C, packed)]
struct Pointer {
    limit: u16,
    base: u32,
}

pub fn init_idt() {
    unsafe {
        let idt = &mut *(&raw mut IDT);
        idt[3] = gate(breakpoint as *const () as usize, 3);
        idt[6] = gate(invalid_opcode as *const () as usize, 0);
        idt[8] = gate(double_fault as *const () as usize, 0);
        idt[13] = gate(gpf as *const () as usize, 0);
        idt[14] = gate(page_fault as *const () as usize, 0);
        idt[Irq::Timer as usize] = gate(timer_irq as *const () as usize, 0);
        idt[Irq::Keyboard as usize] = gate(keyboard_irq as *const () as usize, 0);
        idt[Irq::Serial as usize] = gate(serial_irq as *const () as usize, 0);
        idt[0x80] = gate(crate::syscall::syscall_entry as *const () as usize, 3);
        let ptr = Pointer { limit: (256 * 8 - 1) as u16, base: &raw const IDT as u32 };
        asm!("lidt [{}]", in(reg) &ptr, options(nostack));
    }
}

unsafe fn io_wait() {
    Port::<u8>::new(0x80).write(0);
}

/// Remap the two 8259 PICs to vectors 32..47.
pub fn init_pics() {
    unsafe {
        let (mut c1, mut d1, mut c2, mut d2) =
            (Port::<u8>::new(0x20), Port::<u8>::new(0x21), Port::<u8>::new(0xA0), Port::<u8>::new(0xA1));
        c1.write(0x11); io_wait();
        c2.write(0x11); io_wait();
        d1.write(PIC_1_OFFSET); io_wait();
        d2.write(PIC_2_OFFSET); io_wait();
        d1.write(4); io_wait();
        d2.write(2); io_wait();
        d1.write(1); io_wait();
        d2.write(1); io_wait();
        // Unmask timer (0), keyboard (1), cascade (2), COM1 (4).
        d1.write(0b1110_1000);
        d2.write(0xFF);
    }
}

fn eoi(_irq: Irq) {
    unsafe { Port::<u8>::new(0x20).write(0x20) };
}

fn cr2() -> usize {
    let v: usize;
    unsafe { asm!("mov {}, cr2", out(reg) v, options(nomem, nostack)) };
    v
}

extern "x86-interrupt" fn breakpoint(frame: Frame) {
    println!("[trap] breakpoint (int3) at {:#x} - handled, resuming", frame.eip);
}

extern "x86-interrupt" fn invalid_opcode(frame: Frame) {
    panic!("invalid opcode at {:#x}", frame.eip);
}

extern "x86-interrupt" fn gpf(frame: Frame, code: usize) {
    if frame.cs & 3 == 3 {
        println!("[trap] general protection fault in user program (code {:#x}) - program stopped", code);
        unsafe { crate::syscall::abort_user(-11) }
    }
    panic!("general protection fault (code {:#x}) at {:#x}", code, frame.eip);
}

extern "x86-interrupt" fn page_fault(frame: Frame, code: usize) {
    let addr = cr2();
    if code & 4 != 0 {
        println!("[trap] page fault in user program at {:#x} (code {:#x}) - program stopped", addr, code);
        unsafe { crate::syscall::abort_user(-14) }
    }
    panic!("page fault at {:#x} (code {:#x}), eip {:#x}", addr, code, frame.eip);
}

extern "x86-interrupt" fn double_fault(frame: Frame, _code: usize) -> ! {
    panic!("DOUBLE FAULT at {:#x}", frame.eip);
}

extern "x86-interrupt" fn timer_irq(_frame: Frame) {
    timer::tick();
    eoi(Irq::Timer);
}

extern "x86-interrupt" fn keyboard_irq(_frame: Frame) {
    let scancode = unsafe { Port::<u8>::new(0x60).read() };
    input::push_scancode(scancode);
    eoi(Irq::Keyboard);
}

extern "x86-interrupt" fn serial_irq(_frame: Frame) {
    while let Some(b) = serial::try_read() {
        input::push_serial(b);
    }
    eoi(Irq::Serial);
}
