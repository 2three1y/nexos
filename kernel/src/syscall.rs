//! System calls: ring-3 programs execute `int 0x80` with the call number in
//! RAX and arguments in RDI, RSI, RDX; the result comes back in RAX.
//!
//!   0  exit(code)            never returns to the program
//!   1  write(ptr, len)       print text (screen + serial), returns len
//!   2  uptime_ms()           milliseconds since boot
//!   3  getpid()              process id (1 for now)
//!   4  read_key()            wait for one key, returns its byte (Enter = 10)
//!   5  beep(freq_hz, ms)     play a tone on the PC speaker (respects mute)
//!   6  sleep_ms(ms)          pause the program
//!
//! `enter_user` drops to ring 3 with `iretq`; `exit` (or a fault in the
//! program) unwinds straight back to the kernel stack saved by `enter_user`.

use crate::memory::{USER_BASE, USER_LIMIT};
use crate::{print, timer};

core::arch::global_asm!(
    r#"
.section .bss
.align 8
kernel_saved_rsp: .quad 0

.section .text
.global syscall_entry
syscall_entry:
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push rax
    mov rcx, rdx
    mov rdx, rsi
    mov rsi, rdi
    mov rdi, rax
    call syscall_dispatch
    add rsp, 8
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    iretq

/* enter_user(entry, user_stack, user_cs, user_ds) -> exit code */
.global enter_user
enter_user:
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    mov [rip + kernel_saved_rsp], rsp
    mov ds, cx
    mov es, cx
    push rcx
    push rsi
    push 0x202
    push rdx
    push rdi
    iretq

/* exit_to_kernel(code): abandon the user program, return from enter_user */
.global exit_to_kernel
exit_to_kernel:
    mov rsp, [rip + kernel_saved_rsp]
    xor eax, eax
    mov ds, ax
    mov es, ax
    mov rax, rdi
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret
"#
);

extern "C" {
    pub fn syscall_entry();
    pub fn enter_user(entry: u64, stack: u64, cs: u64, ds: u64) -> i64;
    fn exit_to_kernel(code: i64) -> !;
}

/// Stop the running user program and return `code` from `enter_user`.
pub unsafe fn abort_user(code: i64) -> ! {
    exit_to_kernel(code)
}

fn user_range_ok(ptr: u64, len: u64) -> bool {
    ptr >= USER_BASE && len <= 4096 && ptr.checked_add(len).is_some_and(|end| end <= USER_LIMIT)
}

#[no_mangle]
extern "C" fn syscall_dispatch(nr: u64, a1: u64, a2: u64, _a3: u64) -> i64 {
    match nr {
        0 => unsafe { exit_to_kernel(a1 as i64) },
        1 => {
            if !user_range_ok(a1, a2) {
                return -14; // EFAULT
            }
            let bytes = unsafe { core::slice::from_raw_parts(a1 as *const u8, a2 as usize) };
            match core::str::from_utf8(bytes) {
                Ok(s) => {
                    print!("{}", s);
                    a2 as i64
                }
                Err(_) => -22, // EINVAL
            }
        }
        2 => timer::uptime_ms() as i64,
        3 => 1,
        4 => crate::input::read_key() as i64,
        5 => {
            // The tone waits on timer ticks, so let interrupts in while it plays.
            x86_64::instructions::interrupts::enable();
            timer::tone((a1 as u32).clamp(20, 20_000), a2.min(3000));
            0
        }
        6 => {
            x86_64::instructions::interrupts::enable();
            timer::sleep_ms(a1.min(10_000));
            0
        }
        _ => -38, // ENOSYS
    }
}
