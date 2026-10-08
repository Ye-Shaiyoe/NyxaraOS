# ==============================================================================
# Nyxara OS - Makefile
# Hybrid C and Rust Operating System Build System
# ==============================================================================

export PATH := /home/akrom/.cargo/bin:/home/akrom/.local/bin:$(PATH)

# Toolchain
ASM      := nasm
CC       := gcc
LD       := ld
OBJCOPY  := objcopy
RUSTC    := rustc

# Directories
BUILD_DIR := build
BOOT_DIR  := boot
HAL_DIR   := hal
KERN_DIR  := kernel
RUST_DIR  := rust
USER_DIR  := userland

# Target Files
OS_IMAGE   := nyxara.img
DISK_IMG   := nyxara_disk.img
BOOT_BIN   := $(BUILD_DIR)/boot.bin
KERNEL_BIN := $(BUILD_DIR)/kernel.bin
KERNEL_ELF := $(BUILD_DIR)/kernel.elf
RUST_LIB   := $(BUILD_DIR)/libnyxara_rust.a
USER_ELFS  := $(BUILD_DIR)/forktest.elf

# Flags
ASM_FLAGS  := -f elf32
C_FLAGS    := -m32 -mno-sse -mno-mmx -mno-sse2 -ffreestanding -fno-pie \
              -fno-stack-protector -fno-builtin -nostdlib -nostdinc \
              -Wall -Wextra -O2 -I$(HAL_DIR) -c
RUST_TARGET:= i686-unknown-linux-gnu
RUST_FLAGS := --target $(RUST_TARGET) --crate-type staticlib -C panic=abort \
              -C relocation-model=static -C opt-level=2 \
              -C target-feature=-sse,-sse2,-sse4.1,-sse4.2,-avx \
              -C llvm-args=-stackrealign
LD_FLAGS   := -m elf_i386 -T linker.ld -nostdlib

# C Objects
C_OBJS := $(BUILD_DIR)/string.o \
          $(BUILD_DIR)/io.o \
          $(BUILD_DIR)/vga.o \
          $(BUILD_DIR)/gdt.o \
          $(BUILD_DIR)/idt.o \
          $(BUILD_DIR)/isr.o \
          $(BUILD_DIR)/timer.o \
          $(BUILD_DIR)/keyboard.o \
          $(BUILD_DIR)/serial.o \
          $(BUILD_DIR)/pci.o \
          $(BUILD_DIR)/rtl8139.o \
          $(BUILD_DIR)/rtc.o \
          $(BUILD_DIR)/fb.o \
          $(BUILD_DIR)/mouse.o \
          $(BUILD_DIR)/ata.o \
          $(BUILD_DIR)/kmain.o

# Rust Source Files
RUST_SRCS := $(shell find $(RUST_DIR)/src fs -name '*.rs')

# Default Target
all: $(OS_IMAGE) $(DISK_IMG)

.PHONY: test-line-editor test-vfs

test-line-editor: | $(BUILD_DIR)
	$(RUSTC) --edition 2021 --test tests/line_editor_test.rs -o $(BUILD_DIR)/line_editor_tests
	$(BUILD_DIR)/line_editor_tests

test-vfs: | $(BUILD_DIR)
	$(RUSTC) --edition 2021 --test tests/vfs_test.rs -o $(BUILD_DIR)/vfs_tests
	$(BUILD_DIR)/vfs_tests

# Ensure build directory exists
$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# 1. Build MBR Bootloader (16-bit binary)
$(BOOT_BIN): $(BOOT_DIR)/boot.asm | $(BUILD_DIR)
	$(ASM) -f bin $< -o $@

# 2. Build Assembly Kernel Entry and ISR Trampolines
$(BUILD_DIR)/kernel_entry.o: $(BOOT_DIR)/kernel_entry.asm | $(BUILD_DIR)
	$(ASM) $(ASM_FLAGS) $< -o $@

# 3. Build C HAL Driver Objects
$(BUILD_DIR)/string.o: $(HAL_DIR)/string.c $(HAL_DIR)/types.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/io.o: $(HAL_DIR)/io.c $(HAL_DIR)/io.h $(HAL_DIR)/types.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@
$(BUILD_DIR)/vga.o: $(HAL_DIR)/vga.c $(HAL_DIR)/vga.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/gdt.o: $(HAL_DIR)/gdt.c $(HAL_DIR)/gdt.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/idt.o: $(HAL_DIR)/idt.c $(HAL_DIR)/idt.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/isr.o: $(HAL_DIR)/isr.c $(HAL_DIR)/isr.h $(HAL_DIR)/vga.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/timer.o: $(HAL_DIR)/timer.c $(HAL_DIR)/timer.h $(HAL_DIR)/isr.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/keyboard.o: $(HAL_DIR)/keyboard.c $(HAL_DIR)/keyboard.h $(HAL_DIR)/isr.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/serial.o: $(HAL_DIR)/serial.c $(HAL_DIR)/serial.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/pci.o: $(HAL_DIR)/pci.c $(HAL_DIR)/pci.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/rtl8139.o: $(HAL_DIR)/rtl8139.c $(HAL_DIR)/rtl8139.h $(HAL_DIR)/pci.h $(HAL_DIR)/isr.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/rtc.o: $(HAL_DIR)/rtc.c $(HAL_DIR)/rtc.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/fb.o: $(HAL_DIR)/fb.c $(HAL_DIR)/fb.h $(HAL_DIR)/serial.h $(HAL_DIR)/types.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/mouse.o: $(HAL_DIR)/mouse.c $(HAL_DIR)/mouse.h $(HAL_DIR)/io.h $(HAL_DIR)/isr.h $(HAL_DIR)/serial.h $(HAL_DIR)/types.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/ata.o: $(HAL_DIR)/ata.c $(HAL_DIR)/ata.h $(HAL_DIR)/io.h $(HAL_DIR)/serial.h $(HAL_DIR)/types.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

# 4. Build C Kernel Main
$(BUILD_DIR)/kmain.o: $(KERN_DIR)/kmain.c $(HAL_DIR)/hal.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

# 5. Build Userland ELF Programs
$(BUILD_DIR)/forktest.elf: $(USER_DIR)/forktest.asm $(USER_DIR)/user.ld | $(BUILD_DIR)
	$(ASM) -f elf32 $< -o $(BUILD_DIR)/forktest.o
	$(LD) -m elf_i386 -T $(USER_DIR)/user.ld -nostdlib -o $@ $(BUILD_DIR)/forktest.o

# Automatically compile all .nyx files and generate initrd.rs
.PHONY: generate-initrd
generate-initrd: $(BUILD_DIR)/forktest.elf | $(BUILD_DIR)
	python3 tools/generate_initrd.py

# 6. Build Rust Static Library
$(RUST_LIB): $(RUST_SRCS) generate-initrd $(RUST_DIR)/Cargo.toml Makefile | $(BUILD_DIR)
	$(RUSTC) $(RUST_FLAGS) $(RUST_DIR)/src/lib.rs -o $@

# 7. Link Assembly, C HAL, and Rust staticlib into Kernel ELF
$(KERNEL_ELF): $(BUILD_DIR)/kernel_entry.o $(C_OBJS) $(RUST_LIB) linker.ld
	$(LD) $(LD_FLAGS) $(BUILD_DIR)/kernel_entry.o $(C_OBJS) $(RUST_LIB) -o $@

# 8. Convert Kernel ELF to Raw Flat Binary
$(KERNEL_BIN): $(KERNEL_ELF)
	$(OBJCOPY) -O binary $< $@

# 9. Create Floppy Disk Image (1.44MB)
$(OS_IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(OS_IMAGE)
	truncate -s 1474560 $(OS_IMAGE)
	@echo "\n>>> Nyxara OS Image successfully built: $(OS_IMAGE) (1.44 MB) <<<\n"

# 10. Create Persistent NyxFS Virtual Hard Disk (1.44MB)
$(DISK_IMG):
	dd if=/dev/zero of=$(DISK_IMG) bs=512 count=2880
	@echo "\n>>> Created persistent storage image: $(DISK_IMG) (1.44 MB) <<<\n"

QEMU_DRIVES := -drive file=$(OS_IMAGE),format=raw,index=0,media=disk -drive file=$(DISK_IMG),format=raw,index=1,media=disk
QEMU_NET    := -netdev user,id=net0 -device rtl8139,netdev=net0
QEMU_VGA    := -vga std

# Run in QEMU (GUI)
run: clean $(DISK_IMG)
	$(MAKE) $(OS_IMAGE)
	qemu-system-i386 $(QEMU_DRIVES) $(QEMU_VGA) $(QEMU_NET)

# Run in QEMU with Serial output directed to terminal stdio
run-serial: clean $(DISK_IMG)
	$(MAKE) $(OS_IMAGE)
	qemu-system-i386 $(QEMU_DRIVES) $(QEMU_VGA) -serial stdio $(QEMU_NET)

# Run in QEMU with Curses text console mode
run-curses: clean $(DISK_IMG)
	$(MAKE) $(OS_IMAGE)
	qemu-system-i386 $(QEMU_DRIVES) -curses $(QEMU_NET)

# Run in QEMU with GDB Debug Server (waiting on port 1234)
debug: $(OS_IMAGE) $(DISK_IMG)
	qemu-system-i386 $(QEMU_DRIVES) -s -S -serial stdio $(QEMU_NET)

# Clean build artifacts (keeps nyxara_disk.img intact for persistence)
clean:
	rm -rf $(BUILD_DIR) $(OS_IMAGE) *.bin *.o akromos.img

# Wipe persistent disk image
clean-disk:
	rm -f $(DISK_IMG)

.PHONY: all run run-serial run-curses debug clean clean-disk