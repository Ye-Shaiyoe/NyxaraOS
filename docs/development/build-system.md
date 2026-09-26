# Build System & Linker Configuration

Nyxara OS utilizes a multi-language build system orchestrated by GNU Make (`Makefile`) and a custom GNU Linker script (`linker.ld`). It compiles NASM Assembly, freestanding C, and freestanding Rust static libraries into a unified flat 32-bit floppy disk image.

## Build Pipeline Overview

```
[boot/boot.asm] ---------- (nasm -f bin) -------------> [build/boot.bin] (512 B)
                                                                 |
[boot/kernel_entry.asm] -- (nasm -f elf32) ---------> [kernel_entry.o]
[hal/*.c, kernel/*.c] ---- (gcc -m32 freestanding) --> [*.o]      |
[rust/src/*.rs] ---------- (rustc i686 staticlib) ---> [libnyxara_rust.a]
                                                                 |
                                                          (ld -T linker.ld)
                                                                 |
                                                                 v
                                                        [build/kernel.elf]
                                                                 |
                                                        (objcopy -O binary)
                                                                 |
                                                                 v
                                                        [build/kernel.bin]
                                                                 |
                                                      (cat & truncate 1.44MB)
                                                                 |
                                                                 v
                                                            [nyxara.img]
```

## Compiler Flags & Toolchain Setup

### 1. NASM Flags
- MBR: `-f bin` generates flat 16-bit machine code.
- Kernel Entry: `-f elf32` outputs 32-bit ELF relocatable object files.

### 2. Freestanding GCC Flags
```makefile
C_FLAGS := -m32 -mno-sse -mno-mmx -mno-sse2 -ffreestanding -fno-pie \
           -fno-stack-protector -fno-builtin -nostdlib -nostdinc \
           -Wall -Wextra -O2 -I$(HAL_DIR) -c
```
- `-ffreestanding -nostdinc -nostdlib`: Disables host headers and runtime libraries.
- `-fno-pie -fno-stack-protector`: Prevents generation of position-independent tables and canary guards.
- `-mno-sse -mno-mmx -mno-sse2`: Ensures C compiler generates pure x87/integer instructions during low-level setup.

### 3. Rustc Static Library Flags
```makefile
RUST_TARGET := i686-unknown-linux-gnu
RUST_FLAGS  := --target $(RUST_TARGET) --crate-type staticlib -C panic=abort \
               -C relocation-model=static -C opt-level=2 \
               -C target-feature=-sse,-sse2,-sse4.1,-sse4.2,-avx \
               -C llvm-args=-stackrealign
```
- `--crate-type staticlib`: Generates a standard `.a` archive containing all symbols.
- `-C panic=abort`: Disables unwinding tables, drastically reducing binary footprint.
- `-C llvm-args=-stackrealign`: Forces LLVM to realign stack frames to 16 bytes for ABI compliance.

## Linker Script Architecture (`linker.ld`)

```ld
OUTPUT_FORMAT(elf32-i386)
OUTPUT_ARCH(i386)
ENTRY(_start)

SECTIONS
{
    . = 0x10000;
    kernel_start = .;

    .text : ALIGN(4K)
    {
        *(.text._start)
        *(.text)
        *(.text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata)
        *(.rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data)
        *(.data.*)
    }

    .bss : ALIGN(4K)
    {
        bss_start = .;
        *(COMMON)
        *(.bss)
        *(.bss.*)
        bss_end = .;
    }

    . = ALIGN(4K);
    kernel_end = .;

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.note.*)
    }
}
```

- **Load Base `0x10000` (64 KB)**: Matches the MBR chunk loader target segment.
- **Section Alignment**: All sections (`.text`, `.rodata`, `.data`, `.bss`) are aligned to 4 KB page boundaries, ensuring clean page table protection boundaries.
- **Linker Boundary Symbols**: `kernel_start`, `kernel_end`, `bss_start`, and `bss_end` allow the PMM and kernel entry routines to identify kernel boundaries dynamically.

## Floppy Image Generation (`nyxara.img`)

The final OS media image is constructed by concatenating the bootloader binary and the kernel binary, then truncating to exactly 1,474,560 bytes (standard 1.44 MB 3.5-inch floppy disk):

```makefile
$(OS_IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(OS_IMAGE)
	truncate -s 1474560 $(OS_IMAGE)
```

## Makefile Targets

| Target | Command | Description |
|---|---|---|
| **Build OS** | `make` or `make all` | Compiles and links the complete `nyxara.img` |
| **Run GUI** | `make run` | Starts QEMU with standard VGA window and RTL8139 NIC |
| **Run Serial** | `make run-serial` | Starts QEMU with COM1 output redirected to terminal stdio |
| **Run Curses** | `make run-curses` | Starts QEMU directly inside terminal curses mode |
| **Debug Mode** | `make debug` | Starts QEMU suspended (`-s -S`) waiting for GDB on port 1234 |
| **Clean** | `make clean` | Removes `build/`, `nyxara.img`, and intermediate binaries |

## Test Targets

The Makefile currently exposes two host-run Rust test targets:

| Target | Test file | Coverage |
|---|---|---|
| `make test-line-editor` | `tests/line_editor_test.rs` | Line editor behavior |
| `make test-vfs` | `tests/vfs_test.rs` | In-memory VFS behavior |

There is no aggregate `make test` target. The kernel also runs a VMM self-test during boot; QEMU smoke-test expectations are documented in [Testing Nyxara](testing.md).
