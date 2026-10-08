# Roadmap

NexOS is the operating system and its kernel.
The long-term goal is to boot straight into the Looscid workspace app, local-first.

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
- [x] The `nexos>` shell, with an app registry as the launch hook for future apps
- [x] PC-speaker boot chime with `mute`
- [x] Insomnia app (built in): moon + starfield, sheep counter, lullaby, Esc back to the shell

## Done (v0.3): Insomnia, ported from the web version

- [x] Insomnia menu: 1 Sheep, 2 Thoughts, 3 Sounds, 4 Goodnight, every screen announced on serial
- [x] 4am Thoughts notepad, saved with the time (CMOS clock) to `thoughts.txt` in the filesystem
- [x] PC-speaker soundscapes in the background (Rain, Fan, Crickets, Old PC, Lullaby) with tempo, intensity and mute
- [x] Goodnight: a gentle chime and "It's now safe to turn off your brain", then back to the shell
- [x] `make run` plays the PC speaker on your sound card; `make wav` records it

## Done (v0.4): the App Store and built-in apps

- [x] App manifest format (`kernel/catalog/*.app`, documented in [docs/APPS.md](docs/APPS.md)), shared with the Linux edition later
- [x] App Store (`store`): list, info, install, uninstall, update, search, installed, open. Offline catalog bundled in the image, installed list in `system/installed.txt`
- [x] Install, uninstall, error and alarm chimes on the PC speaker, always with words
- [x] Home menu (`home`): installed apps, numbered
- [x] Native app toolkit (`apps/ui.rs`): same layout and keys in every app (h help, q quit, Esc)
- [x] Preinstalled: Notes, Calculator, Clock (timer, stopwatch, alarm), System Info, Insomnia, Hello
- [x] In the store: Piano, Guess the Number
- [x] Keyboard input and sound for user programs: `read_key`, `beep`, `sleep_ms` system calls
- [x] `nexos` app API crate: the first version of the app API for ring-3 programs (Guess the Number uses it)

## Done (v0.4.1): one name

- [x] The OS is called NexOS everywhere: boot banner, `nexos>` prompt, about, apps and docs

## Done (v0.5): 32-bit and in the browser

- [x] i686 build (`make iso32`): 32-bit boot stub, GDT/TSS, IDT, two-level paging, `int 0x80` and ring 3, sharing everything else with x86_64
- [x] The real 32-bit kernel boots in the browser with v86, with an accessible serial console and PC-speaker sound: https://2three1y.github.io/nexos/real/
- [ ] Try it on real old hardware (a Pentium-class PC or an early netbook)

## Next: the OS layer

- [ ] Per-process address spaces (a page table per program) and memory protection between programs
- [ ] Processes + a preemptive scheduler on the timer interrupt
- [ ] `syscall`/`sysret` fast path, more system calls (open, read/write files, spawn)
- [ ] A block device driver (ATA/virtio) and a simple on-disk filesystem
- [ ] Load apps from disk instead of embedding them in the kernel, and keep the installed list on disk
- [ ] App Store packages that carry their own program (install copies an ELF to disk), then a network catalog
- [ ] Framebuffer graphics console (UEFI boot via GOP), keeping the serial mirror

## Future apps

- [ ] Insomnia as a ring-3 app: move it out of the kernel once user programs can read the keyboard and draw
- [ ] Real audio (Sound Blaster 16 or Intel HD Audio) so Sounds can mix several layers with real volume sliders, like the web version
- [ ] Save `thoughts.txt` to disk once there is an on-disk filesystem

## Two editions of NexOS

NexOS will ship in two editions that share one user-facing layer:

- **NexOS kernel edition** (the priority): the full OS on our own kernel, this repo.
- **NexOS Linux edition** (later): a distro on the Linux kernel, so people can install it on real laptops today.

Plan: the shell and apps target a small **NexOS API** (a syscall-like interface), with two backends: NexOS system calls and Linux/POSIX. Apps written once run on both editions.

- [x] App manifests both editions read (docs/APPS.md)
- [ ] Define the full NexOS API (files, input, screen, sound, time, processes). Input, sound and time are in the `nexos` crate today
- [x] NexOS backend for input, output, sound and time (syscalls)
- [ ] Linux/POSIX backend, then a Linux-edition image

## Then: an SDK and the Looscid workspace

- [ ] Grow the `nexos` crate in `userland/` into a full SDK so apps can be written against NexOS
- [ ] Move the native apps (Notes, Calculator, Clock...) to ring 3 once there are file system calls
- [ ] Networking (virtio-net) for sync between devices
- [ ] Boot into the Looscid workspace app as the first real app

## Always

- Keyboard-first, high contrast, and everything readable over the serial console.
