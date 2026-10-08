//! Multiboot2 header and the 32-bit entry stub (i686 build).
//!
//! GRUB enters `_start` in 32-bit protected mode with paging off, EAX holding
//! the Multiboot2 magic and EBX the physical address of the boot information.
//! We identity-map the first 1 GiB with 4 MiB pages (PSE, classic 2-level
//! paging, no PAE so it runs on any i686-class CPU), turn paging on and call
//! `kernel_main(magic, info_addr)` (cdecl). There is no mode switch: the
//! 32-bit kernel stays in protected mode.

core::arch::global_asm!(
    r#"
.section .multiboot_header, "a"
.align 8
mb2_start:
    .long 0xe85250d6                 /* magic */
    .long 0                          /* architecture: i386 protected mode */
    .long mb2_end - mb2_start        /* header length */
    .long 0x100000000 - (0xe85250d6 + (mb2_end - mb2_start))  /* checksum */
    .align 8
    .short 0                         /* end tag */
    .short 0
    .long 8
mb2_end:

.section .bss
.align 4096
.global boot_pd
boot_pd: .skip 4096
.global boot_stack_top
boot_stack_bottom: .skip 65536
boot_stack_top:

.section .text.boot, "ax"
.global _start
_start:
    cli
    cld
    movl $boot_stack_top, %esp
    movl %eax, %esi                  /* multiboot2 magic */
    movl %ebx, %edi                  /* boot info address */

    /* pd[i] -> i * 4 MiB, present | writable | 4 MiB page, for i < 256 (1 GiB) */
    xorl %ecx, %ecx
1:
    movl %ecx, %eax
    shll $22, %eax
    orl $0x83, %eax
    movl %eax, boot_pd(,%ecx,4)
    incl %ecx
    cmpl $256, %ecx
    jne 1b

    movl $boot_pd, %eax
    movl %eax, %cr3
    movl %cr4, %eax
    orl $(1<<4), %eax                /* PSE: 4 MiB pages */
    movl %eax, %cr4
    movl %cr0, %eax
    orl $((1<<31) | (1<<16)), %eax   /* paging + write-protect */
    movl %eax, %cr0

    pushl %edi                       /* kernel_main(magic, info) */
    pushl %esi
    call kernel_main
2:
    hlt
    jmp 2b
"#,
    options(att_syntax)
);
