# NexOS — build and run.
#
#   make        build userland + kernel and a bootable GRUB ISO
#   make run    boot it in QEMU (VGA window + serial on this terminal + PC speaker sound)
#   make serial boot headless: serial console only (screen-reader friendly)
#   make wav    boot headless and record the PC speaker to build/speaker.wav
#
# 32-bit (i686) build, for older PCs and for the in-browser emulator:
#   make ARCH=i686        (or: make iso32)  -> build/nexos-i686.iso
#   make ARCH=i686 run / serial / wav       boot it in qemu-system-i386
#
# Sound: `make run` routes the emulated PC speaker to your sound card
# (coreaudio on macOS, PulseAudio/PipeWire elsewhere). Override with
# `make run AUDIO=sdl`, `AUDIO=alsa`, or `AUDIO=none` for silence.
#   make clean

ARCH     ?= x86_64
PROFILE  := release

ifeq ($(ARCH),i686)
# Custom 32-bit target (soft-float, no SSE, any i686-class CPU); core and
# alloc are rebuilt for it with build-std.
TARGET   := i686-nexos
CARGOFLAGS := --target targets/i686-nexos.json -Zjson-target-spec \
	-Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem
ISO      := build/nexos-i686.iso
ISODIR   := build/iso-i686
QEMU     := qemu-system-i386
# A small BIOS-only ISO (about 1 MB) with just the GRUB modules NexOS needs,
# so it loads fast in the browser emulator and fits on anything.
GRUBFLAGS := --compress=xz --install-modules="multiboot2 normal iso9660 biosdisk" \
	--modules="multiboot2" --locales= --fonts= --themes=
else ifeq ($(ARCH),x86_64)
TARGET   := x86_64-unknown-none
CARGOFLAGS :=
ISO      := build/nexos.iso
ISODIR   := build/iso
QEMU     := qemu-system-x86_64
GRUBFLAGS :=
else
$(error ARCH must be x86_64 or i686)
endif

OUT      := target/$(TARGET)/$(PROFILE)
QEMUFLAGS := -m 128M -no-reboot -serial stdio -rtc base=localtime

ifeq ($(shell uname -s),Darwin)
AUDIO ?= coreaudio
else
AUDIO ?= pa
endif
AUDIOFLAGS := -audiodev $(AUDIO),id=snd0 -machine pc,pcspk-audiodev=snd0

.PHONY: all iso iso32 kernel userland run serial wav clean

all: iso

userland:
	cargo build --release -p nexos-userland $(CARGOFLAGS)

kernel: userland
	cargo build --release -p nexos-kernel $(CARGOFLAGS)

iso: kernel
	@mkdir -p $(ISODIR)/boot/grub
	cp $(OUT)/nexos-kernel $(ISODIR)/boot/nexos-kernel.elf
	cp boot/grub.cfg $(ISODIR)/boot/grub/grub.cfg
	grub-mkrescue $(GRUBFLAGS) -o $(ISO) $(ISODIR) 2>/dev/null
	@echo "==> $(ISO) ready"

# Shorthand for the 32-bit ISO.
iso32:
	$(MAKE) ARCH=i686 iso

run: iso
	$(QEMU) $(QEMUFLAGS) $(AUDIOFLAGS) -cdrom $(ISO)

serial: iso
	$(QEMU) $(QEMUFLAGS) -display none -cdrom $(ISO)

wav: iso
	$(QEMU) $(QEMUFLAGS) -display none -audiodev wav,id=snd0,path=build/speaker.wav \
		-machine pc,pcspk-audiodev=snd0 -cdrom $(ISO)

clean:
	cargo clean
	rm -rf build
