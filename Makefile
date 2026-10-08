# Looscid OS (NexOS kernel) — build and run.
#
#   make        build userland + kernel and a bootable GRUB ISO
#   make run    boot it in QEMU (VGA window + serial on this terminal)
#   make serial boot headless: serial console only (screen-reader friendly)
#   make clean

TARGET   := x86_64-unknown-none
PROFILE  := release
OUT      := target/$(TARGET)/$(PROFILE)
ISO      := build/looscid.iso
QEMU     := qemu-system-x86_64
QEMUFLAGS := -m 128M -no-reboot -serial stdio

.PHONY: all iso kernel userland run serial clean

all: iso

userland:
	cargo build --release -p nexos-userland

kernel: userland
	cargo build --release -p nexos-kernel

iso: kernel
	@mkdir -p build/iso/boot/grub
	cp $(OUT)/nexos-kernel build/iso/boot/nexos-kernel.elf
	cp boot/grub.cfg build/iso/boot/grub/grub.cfg
	grub-mkrescue -o $(ISO) build/iso 2>/dev/null
	@echo "==> $(ISO) ready"

run: iso
	$(QEMU) $(QEMUFLAGS) -cdrom $(ISO)

serial: iso
	$(QEMU) $(QEMUFLAGS) -display none -cdrom $(ISO)

clean:
	cargo clean
	rm -rf build
