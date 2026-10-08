//! Multiboot2 header and the 32-bit -> 64-bit entry stub.
//!
//! GRUB enters `_start` in 32-bit protected mode with paging off, EAX holding
//! the Multiboot2 magic and EBX the physical address of the boot information.
//! We identity-map the first 1 GiB with 2 MiB pages, enable PAE + long mode +
//! paging, load a minimal 64-bit GDT and far-jump into 64-bit code, which
//! calls `kernel_main(magic, info_addr)`.

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
.global boot_p4
boot_p4: .skip 4096
boot_p3: .skip 4096
boot_p2: .skip 4096
.global boot_stack_top
boot_stack_bottom: .skip 65536
boot_stack_top:

.section .rodata
.align 8
boot_gdt64:
    .quad 0
    .quad (1<<43) | (1<<44) | (1<<47) | (1<<53)   /* 64-bit code segment */
boot_gdt64_ptr:
    .short boot_gdt64_ptr - boot_gdt64 - 1
    .quad boot_gdt64

.section .text.boot, "ax"
.code32
.global _start
_start:
    cli
    movl $boot_stack_top, %esp
    movl %eax, %esi                  /* multiboot2 magic */
    movl %ebx, %edi                  /* boot info address */

    /* p4[0] -> p3, p3[0] -> p2 */
    movl $boot_p3, %eax
    orl $3, %eax
    movl %eax, boot_p4
    movl $boot_p2, %eax
    orl $3, %eax
    movl %eax, boot_p3

    /* p2[i] -> i * 2 MiB, present | writable | huge */
    xorl %ecx, %ecx
1:
    movl %ecx, %eax
    shll $21, %eax
    orl $0x83, %eax
    movl %eax, boot_p2(,%ecx,8)
    movl $0, boot_p2+4(,%ecx,8)
    incl %ecx
    cmpl $512, %ecx
    jne 1b

    movl $boot_p4, %eax
    movl %eax, %cr3
    movl %cr4, %eax
    orl $(1<<5), %eax                /* PAE */
    movl %eax, %cr4
    movl $0xC0000080, %ecx           /* EFER */
    rdmsr
    orl $((1<<8) | (1<<11)), %eax    /* long mode enable + no-execute */
    wrmsr
    movl %cr0, %eax
    orl $(1<<31), %eax               /* paging */
    movl %eax, %cr0

    lgdt boot_gdt64_ptr
    ljmp $0x08, $long_mode_start

.code64
long_mode_start:
    xorw %ax, %ax
    movw %ax, %ss
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs
    movabsq $boot_stack_top, %rsp
    movl %edi, %edi                  /* zero-extend info address */
    movl %esi, %esi
    xchgq %rdi, %rsi                 /* kernel_main(magic, info) */
    call kernel_main
2:
    hlt
    jmp 2b
"#,
    options(att_syntax)
);
