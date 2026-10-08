# Looscid OS (NexOS kernel)

Looscid OS is a hobby operating system for x86_64, written in Rust.
**NexOS** is its kernel; **Looscid OS** is the system layer that runs on top of it.

It boots on real BIOS hardware or in QEMU, drops you into the `looscid>` shell,
and can already run a program in user mode (ring 3) that talks to the kernel
through system calls.

```
NexOS kernel v0.2.0 (x86_64) - booting Looscid OS
[ ok ] boot loader: GRUB 2.06
[ ok ] serial console on COM1 (everything on screen is mirrored here)
[ ok ] GDT loaded: kernel + user segments, TSS with double-fault stack
[ ok ] IDT loaded: breakpoint, page fault, GPF, double fault, syscall gate 0x80
[ ok ] memory: 127 MiB usable RAM, frame allocator ready
[ ok ] paging: mapped 256 pages for a 1 MiB heap at 0x444444440000
[ ok ] heap test: Box, Vec and String work
[ ok ] PIC remapped, PIT timer at 100 Hz, interrupts on
[ ok ] input: PS/2 keyboard + serial (COM1) keyboard
[ ok ] breakpoint exception handled, execution resumed
[ ok ] in-memory filesystem: 3 files
[ ok ] user mode: 1 app(s) registered, syscalls via int 0x80
Welcome to Looscid OS.
looscid> run hello
Hello from ring 3! I'm the first Looscid userland program.
```

## Features

**NexOS kernel**
- Boots with GRUB via Multiboot2; a small 32-bit stub sets up page tables and switches to 64-bit long mode
- GDT + TSS (separate kernel and user segments, a dedicated double-fault stack)
- IDT with handlers for breakpoint, invalid opcode, page fault, general protection fault and double fault
- 8259 PIC + PIT timer at 100 Hz (`uptime`)
- PS/2 keyboard input (US layout), plus keyboard input over the serial port
- Physical frame allocator built from the Multiboot2 memory map
- Paging: the kernel maps new pages through the live page tables (heap, user programs)
- 1 MiB kernel heap (`alloc`: `Box`, `Vec`, `String`)
- System calls through `int 0x80`: `exit`, `write`, `uptime_ms`, `getpid`
- Loads ELF programs into a user address window and runs them in ring 3. A crash in a program stops only that program, not the kernel
- PC-speaker boot chime (`beep`, `mute`)
- A panic handler with a recursion lock that reports file and line

**Looscid OS**
- The `looscid>` shell: `help`, `clear`, `about`, `uptime`, `echo`, `mem`, `ls`, `cat`, `write`, `rm`, `apps`, `run`, `beep`, `mute`, `theme`, `int3`, `reboot`
- An in-memory filesystem (`ls`, `cat`, `write`, `rm`)
- `userland/hello`: the first ring-3 Looscid program, embedded in the kernel image
- An app registry (`apps`, `run <app>`): this is where Looscid apps will plug in

## Accessibility

- **Keyboard only.** Nothing needs a mouse.
- **Screen-reader friendly serial console.** Everything printed to the screen is also printed to COM1, and the shell accepts typing from the serial line too. You can use the whole OS from a terminal with a screen reader (`make serial`). The serial output contains no ANSI escape codes, so nothing gets read aloud as noise.
- **High contrast.** Bright white on black by default. `theme light` switches to black on white.
- **Sound is optional.** The boot chime is short, and `mute` turns all sound off.

## Build and run

You need: a Rust **nightly** toolchain (`rust-toolchain.toml` selects it automatically via rustup), plus
`qemu-system-x86_64`, `grub-mkrescue` (`grub-pc-bin` + `grub-common`), `xorriso` and `mtools`.

```sh
# Debian / Ubuntu
sudo apt install qemu-system-x86 grub-pc-bin grub-common xorriso mtools
curl https://sh.rustup.rs -sSf | sh     # if you don't have rustup yet

make            # build userland + kernel + build/looscid.iso
make run        # boot in QEMU (VGA window, serial log in this terminal)
make serial     # boot headless: serial console only (screen readers)
./build.sh --qemu   # same as make run
```

The ISO also boots on BIOS PCs from a USB stick (`dd` it to the stick).

## Layout

```
nexos/
├── Cargo.toml            # workspace: kernel + userland
├── rust-toolchain.toml   # nightly + rust-src + llvm-tools
├── .cargo/config.toml    # target x86_64-unknown-none (built in, no custom JSON)
├── Makefile, build.sh    # build + ISO + QEMU
├── boot/grub.cfg         # GRUB menu entry (multiboot2)
├── kernel/               # NexOS kernel
│   ├── linker.ld         # loaded at 1 MiB
│   └── src/
│       ├── boot.rs       # Multiboot2 header, 32-bit to 64-bit entry stub
│       ├── main.rs       # kernel_main, boot sequence, panic handler
│       ├── vga.rs        # 80x25 text console, scrolling, cursor, themes
│       ├── serial.rs     # COM1 output mirror + input
│       ├── console.rs    # print!/println! to screen + serial
│       ├── gdt.rs        # GDT, TSS
│       ├── interrupts.rs # IDT, exceptions, PIC, IRQ handlers
│       ├── memory.rs     # memory map, frame allocator, paging
│       ├── allocator.rs  # kernel heap
│       ├── timer.rs      # PIT, uptime, PC speaker
│       ├── input.rs      # keyboard decoding, input queue
│       ├── syscall.rs    # int 0x80 gate, enter/exit user mode
│       ├── user.rs       # ELF loader, app registry
│       ├── fs.rs         # in-memory filesystem
│       └── shell.rs      # the looscid> shell
└── userland/             # Looscid ring-3 programs
    ├── user.ld           # linked at 0x4000_0000
    └── src/main.rs       # `hello`
```

## Roadmap

See [ROADMAP.md](ROADMAP.md): first processes and a scheduler, then on-disk apps, then Looscid's workspace running as the first real app.

## License

MIT. See [LICENSE](LICENSE).

---

Built with Tab (https://tab.bot)
