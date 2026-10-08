//! IDT, CPU exception handlers, the 8259 PIC, and hardware IRQs.
//!
//! Vectors 32..47 are the remapped PIC lines: 32 = PIT timer, 33 = PS/2
//! keyboard, 36 = COM1 serial. Vector 0x80 is the system-call gate (callable
//! from ring 3, see syscall.rs).

use crate::{gdt, input, println, serial, timer};
use pic8259::ChainedPics;
use spin::{Mutex, Once};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::{PrivilegeLevel, VirtAddr};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Irq {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Serial = PIC_1_OFFSET + 4,
}

static IDT: Once<InterruptDescriptorTable> = Once::new();

pub fn init_idt() {
    let idt = IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint);
        idt.invalid_opcode.set_handler_fn(invalid_opcode);
        idt.general_protection_fault.set_handler_fn(gpf);
        idt.page_fault.set_handler_fn(page_fault);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault).set_stack_index(gdt::DOUBLE_FAULT_IST);
        }
        idt[Irq::Timer as u8].set_handler_fn(timer_irq);
        idt[Irq::Keyboard as u8].set_handler_fn(keyboard_irq);
        idt[Irq::Serial as u8].set_handler_fn(serial_irq);
        unsafe {
            idt[0x80]
                .set_handler_addr(VirtAddr::new(crate::syscall::syscall_entry as *const () as u64))
                .set_privilege_level(PrivilegeLevel::Ring3);
        }
        idt
    });
    idt.load();
}

pub fn init_pics() {
    unsafe {
        let mut pics = PICS.lock();
        pics.initialize();
        // Unmask timer (0), keyboard (1), cascade (2), COM1 (4).
        pics.write_masks(0b1110_1000, 0xFF);
    }
}

fn eoi(irq: Irq) {
    unsafe { PICS.lock().notify_end_of_interrupt(irq as u8) };
}

extern "x86-interrupt" fn breakpoint(frame: InterruptStackFrame) {
    println!("[trap] breakpoint (int3) at {:#x} - handled, resuming", frame.instruction_pointer.as_u64());
}

extern "x86-interrupt" fn invalid_opcode(frame: InterruptStackFrame) {
    panic!("invalid opcode at {:#x}", frame.instruction_pointer.as_u64());
}

extern "x86-interrupt" fn gpf(frame: InterruptStackFrame, code: u64) {
    if frame.code_segment.rpl() == PrivilegeLevel::Ring3 {
        println!("[trap] general protection fault in user program (code {:#x}) - program stopped", code);
        unsafe { crate::syscall::abort_user(-11) }
    }
    panic!("general protection fault (code {:#x}) at {:#x}", code, frame.instruction_pointer.as_u64());
}

extern "x86-interrupt" fn page_fault(frame: InterruptStackFrame, code: PageFaultErrorCode) {
    let addr = x86_64::registers::control::Cr2::read_raw();
    if code.contains(PageFaultErrorCode::USER_MODE) {
        println!("[trap] page fault in user program at {:#x} ({:?}) - program stopped", addr, code);
        unsafe { crate::syscall::abort_user(-14) }
    }
    panic!("page fault at {:#x} ({:?}), rip {:#x}", addr, code, frame.instruction_pointer.as_u64());
}

extern "x86-interrupt" fn double_fault(frame: InterruptStackFrame, _code: u64) -> ! {
    panic!("DOUBLE FAULT at {:#x} (handled on its own IST stack)", frame.instruction_pointer.as_u64());
}

extern "x86-interrupt" fn timer_irq(_frame: InterruptStackFrame) {
    timer::tick();
    eoi(Irq::Timer);
}

extern "x86-interrupt" fn keyboard_irq(_frame: InterruptStackFrame) {
    let mut port: x86_64::instructions::port::Port<u8> = x86_64::instructions::port::Port::new(0x60);
    let scancode = unsafe { port.read() };
    input::push_scancode(scancode);
    eoi(Irq::Keyboard);
}

extern "x86-interrupt" fn serial_irq(_frame: InterruptStackFrame) {
    while let Some(b) = serial::try_read() {
        input::push_serial(b);
    }
    eoi(Irq::Serial);
}
