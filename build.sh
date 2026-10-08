#!/usr/bin/env bash
# Looscid OS (NexOS kernel) build script — a thin wrapper around make.
#
# Usage:
#   ./build.sh            # build userland + kernel + bootable ISO (build/looscid.iso)
#   ./build.sh --qemu     # build and boot in QEMU (VGA window + serial here)
#   ./build.sh --serial   # build and boot headless, serial console only
set -e
cd "$(dirname "$0")"

case "$1" in
    --qemu)   make run ;;
    --serial) make serial ;;
    *)        make iso ;;
esac
