# Changelog

## 0.5.0
- NexOS now also builds as a 32-bit i686 kernel for older PCs: `make iso32` (or `make ARCH=i686`) makes `build/nexos-i686.iso`. The shell, filesystem, App Store, apps and ring-3 programs are shared; the 32-bit boot stub, GDT/TSS, IDT, paging (two-level, 4 MiB pages) and system-call entry live in `kernel/src/i686/`. The x86_64 build is unchanged.
- The real 32-bit kernel boots in the browser: https://2three1y.github.io/nexos/real/ runs the ISO in the v86 emulator, with the serial console as an accessible log, a command box and the PC speaker on Web Audio.
- `about` and System Info say which architecture you are running.

## 0.4.1
- The OS is now called NexOS (it was Looscid OS). The prompt is `nexos>`, and the boot banner, `about`, apps and docs all say NexOS.

## 0.4.0
- App Store and built-in apps (Notes, Calculator, Clock, System Info, Insomnia, Hello; Piano and Guess the Number in the store).
