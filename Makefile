# Looscid OS (NexOS kernel) — build and run.
#
#   make        build userland + kernel and a bootable GRUB ISO
#   make run    boot it in QEMU (VGA window + serial on this terminal + PC speaker sound)
#   make serial boot headless: serial console only (screen-reader friendly)
#   make wav    boot headless and record the PC speaker to build/speaker.wav
#
# Sound: `make run` routes the emulated PC speaker to your sound card
# (coreaudio on macOS, PulseAudio/PipeWire elsewhere). Override with
# `make run AUDIO=sdl`, `AUDIO=alsa`, or `AUDIO=none` for silence.
#   make clean

TARGET   := x86_64-unknown-none
PROFILE  := release
OUT      := target/$(TARGET)/$(PROFILE)
ISO      := build/looscid.iso
QEMU     := qemu-system-x86_64
QEMUFLAGS := -m 128M -no-reboot -serial stdio -rtc base=localtime

ifeq ($(shell uname -s),Darwin)
AUDIO ?= coreaudio
else
AUDIO ?= pa
endif
AUDIOFLAGS := -audiodev $(AUDIO),id=snd0 -machine pc,pcspk-audiodev=snd0

.PHONY: all iso kernel userland run serial wav clean

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
	$(QEMU) $(QEMUFLAGS) $(AUDIOFLAGS) -cdrom $(ISO)

serial: iso
	$(QEMU) $(QEMUFLAGS) -display none -cdrom $(ISO)

wav: iso
	$(QEMU) $(QEMUFLAGS) -display none -audiodev wav,id=snd0,path=build/speaker.wav \
		-machine pc,pcspk-audiodev=snd0 -cdrom $(ISO)

clean:
	cargo clean
	rm -rf build
