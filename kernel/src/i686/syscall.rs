//! System calls (i686 build): ring-3 programs execute `int 0x80` with the
//! call number in EAX and arguments in ECX, EDX (and EBX); the result comes
//! back in EAX. Same numbers and meanings as the x86_64 build:
//!
//!   0  exit(code)            never returns to the program
//!   1  write(ptr, len)       print text (screen + serial), returns len
//!   2  uptime_ms()           milliseconds since boot
//!   3  getpid()              process id (1 for now)
//!   4  read_key()            wait for one key, returns its byte (Enter = 10)
//!   5  beep(freq_hz, ms)     play a tone on the PC speaker (respects mute)
//!   6  sleep_ms(ms)          pause the program
//!
//! `enter_user` drops to ring 3 with `iretd`; `exit` (or a fault in the
//! program) unwinds straight back to the kernel stack saved by `enter_user`.

use crate::memory::{USER_BASE, USER_LIMIT};
use crate::{print, timer};

core::arch::global_asm!(
    r#"
.section .bss
.align 4
kernel_saved_esp: .long 0

.section .text
.global syscall_entry
syscall_entry:
    cld
    push ebp
    push edi
    push esi
    push edx
    push ecx
    push ebx
    push ebx
    push edx
    push ecx
    push eax
    call syscall_dispatch
    add esp, 16
    pop ebx
    pop ecx
    pop edx
    pop esi
    pop edi
    pop ebp
    iretd

/* enter_user_raw(entry, user_stack, user_cs, user_ds) -> exit code (cdecl) */
.global enter_user_raw
enter_user_raw:
    push ebp
    push ebx
    push esi
    push edi
    mov [kernel_saved_esp], esp
    mov eax, [esp + 20]
    mov edx, [esp + 24]
    mov ecx, [esp + 28]
    mov ebx, [esp + 32]
    mov ds, bx
    mov es, bx
    mov fs, bx
    mov gs, bx
    push ebx
    push edx
    push 0x202
    push ecx
    push eax
    iretd

/* exit_to_kernel(code): abandon the user program, return from enter_user_raw */
.global exit_to_kernel
exit_to_kernel:
    mov eax, [esp + 4]
    mov esp, [kernel_saved_esp]
    mov cx, 0x10
    mov ds, cx
    mov es, cx
    mov fs, cx
    mov gs, cx
    pop edi
    pop esi
    pop ebx
    pop ebp
    ret
"#
);

extern "C" {
    pub fn syscall_entry();
    fn enter_user_raw(entry: u32, stack: u32, cs: u32, ds: u32) -> i32;
    fn exit_to_kernel(code: i32) -> !;
}

/// Drop to ring 3 at `entry` with stack `stack`; returns the program's exit code.
pub unsafe fn enter_user(entry: u64, stack: u64, cs: u64, ds: u64) -> i64 {
    enter_user_raw(entry as u32, stack as u32, cs as u32, ds as u32) as i64
}

/// Stop the running user program and return `code` from `enter_user`.
pub unsafe fn abort_user(code: i64) -> ! {
    exit_to_kernel(code as i32)
}

fn user_range_ok(ptr: u32, len: u32) -> bool {
    let (ptr, len) = (ptr as u64, len as u64);
    ptr >= USER_BASE && len <= 4096 && ptr + len <= USER_LIMIT
}

#[no_mangle]
extern "C" fn syscall_dispatch(nr: u32, a1: u32, a2: u32, _a3: u32) -> i32 {
    match nr {
        0 => unsafe { exit_to_kernel(a1 as i32) },
        1 => {
            if !user_range_ok(a1, a2) {
                return -14; // EFAULT
            }
            let bytes = unsafe { core::slice::from_raw_parts(a1 as *const u8, a2 as usize) };
            match core::str::from_utf8(bytes) {
                Ok(s) => {
                    print!("{}", s);
                    a2 as i32
                }
                Err(_) => -22, // EINVAL
            }
        }
        2 => timer::uptime_ms() as i32,
        3 => 1,
        4 => crate::input::read_key() as i32,
        5 => {
            // The tone waits on timer ticks, so let interrupts in while it plays.
            crate::cpu::interrupts::enable();
            timer::tone(a1.clamp(20, 20_000), (a2 as u64).min(3000));
            0
        }
        6 => {
            crate::cpu::interrupts::enable();
            timer::sleep_ms((a1 as u64).min(10_000));
            0
        }
        _ => -38, // ENOSYS
    }
}
