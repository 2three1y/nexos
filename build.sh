#!/usr/bin/env bash
# NexOS build script.
#
# Cross-compiles the kernel for the bare x86_64 target (no OS) and produces a
# bootable image for QEMU. The custom target `x86_64-unknown-none` is defined
# in kernel/targets/x86_64-unknown-none.json.
#
# Usage:
#   ./build.sh            # build the kernel (freestanding x86_64)
#   ./build.sh --qemu     # build and boot the kernel image in QEMU
set -e
cd "$(dirname "$0")"

TARGET=x86_64-unknown-none

echo "==> Building kernel for target: $TARGET"
cargo build --target "$TARGET" kernel

if [[ "$1" == "--qemu" ]]; then
    echo "==> Booting kernel image in QEMU"
    qemu-system-x86_64 -drive format=raw,file=kernel/out/nexos-kernel.bin
fi