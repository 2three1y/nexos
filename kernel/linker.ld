/* NexOS kernel linker script: loaded by GRUB at 1 MiB (identity mapped). */
ENTRY(_start)

SECTIONS
{
    . = 1M;

    .boot : ALIGN(8)
    {
        KEEP(*(.multiboot_header))
    }

    .text : ALIGN(4K)
    {
        *(.text.boot)
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
        *(.got .got.*)
    }

    .bss : ALIGN(4K)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    . = ALIGN(4K);
    __kernel_end = .;

    /DISCARD/ : { *(.eh_frame*) *(.comment*) *(.note*) }
}
