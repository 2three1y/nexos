# NexOS

An open-source operating system kernel written in Rust.

## Workspace layout

```
nexos/
├── Cargo.toml          # Cargo workspace (members: kernel, userland)
├── kernel/             # Freestanding no_std x86_64 kernel core
│   └── src/lib.rs      # VGA console, serial stub, panic handler, halt loop
└── userland/           # Ring-3 programs (placeholder)
```

## Building the kernel

The kernel is a **freestanding** crate: it does not link the standard library
and has no host `main`. It is entered via `_start`, which the bootloader/QEMU
jumps to directly.

To typecheck the kernel source as a library (no host linking):

```sh
cd kernel
cargo build --target x86_64-unknown-linux-gnu --lib
```

## Boot intent

A later milestone produces a bootable image (multiboot/ISO) that QEMU loads,
jumping to the kernel's `_start` entry point. The kernel currently prints a
boot banner to the VGA text console (0xB8000) and halts.