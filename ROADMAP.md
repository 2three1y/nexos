# Roadmap

NexOS is the kernel. Looscid OS is the operating system layer on top of it.
The long-term goal is to boot straight into Looscid, the local-first workspace.

## Done (v0.2): verified booting in QEMU

- [x] Bootable image: GRUB + Multiboot2, 32-bit to 64-bit long-mode entry
- [x] GDT + TSS, IDT with exception handlers (breakpoint, page fault, GPF, double fault on its own stack)
- [x] PIC + PIT timer (100 Hz), uptime
- [x] PS/2 keyboard input and serial input; all screen output mirrored to serial
- [x] Physical frame allocator (Multiboot2 memory map) and paging (mapping new pages at runtime)
- [x] Kernel heap (1 MiB, `alloc` crate)
- [x] System call interface (`int 0x80`: exit, write, uptime_ms, getpid)
- [x] Ring-3 user program loaded from an ELF (`userland/` builds `hello`; the shell runs it with `run hello`)
- [x] In-memory filesystem (`ls`, `cat`, `write`, `rm`)
- [x] The `looscid>` shell, with an app registry as the launch hook for future apps
- [x] PC-speaker boot chime with `mute`
- [x] Insomnia app (built in): moon + starfield, sheep counter, lullaby, Esc back to the shell

## Next: the OS layer

- [ ] Per-process address spaces (a page table per program) and memory protection between programs
- [ ] Processes + a preemptive scheduler on the timer interrupt
- [ ] `syscall`/`sysret` fast path, more system calls (read, open, spawn, sleep)
- [ ] Keyboard input for user programs (a `read` syscall)
- [ ] A block device driver (ATA/virtio) and a simple on-disk filesystem
- [ ] Load apps from disk instead of embedding them in the kernel
- [ ] Framebuffer graphics console (UEFI boot via GOP), keeping the serial mirror

## Future apps

- [ ] Insomnia, full version: soundscape mixer, sheep counter and 4am notes, ported from the Insomnia OS web toy (github.com/2three1y/insomnia-os) as a ring-3 Looscid OS app once user programs can read the keyboard and draw

## Then: Looscid

- [ ] A Looscid runtime/SDK in `userland/` so apps can be written against Looscid OS
- [ ] Networking (virtio-net) for sync between devices
- [ ] Boot into the Looscid workspace as the first real app

## Always

- Keyboard-first, high contrast, and everything readable over the serial console.
