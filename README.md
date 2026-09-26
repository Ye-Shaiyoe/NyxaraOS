# Nyxara

**Nyxara** is a modern hybrid operating system built from scratch for **x86 (32-bit Protected Mode)** architecture, combining the reliability of **C & Assembly** at the *Hardware Abstraction Layer (HAL)* level with the memory safety and systemic capabilities of **Rust (`no_std`)** at the *Kernel Core & Shell Subsystem* level.

---

## Preview

<img width="830" height="424" alt="image" src="https://github.com/user-attachments/assets/3d597770-9a72-4257-be91-7878536a452a" />

## Architecture & System Design

```text
                              +-----------------------------+
                              |        Nyxara Shell         |
                              |  (Rust Interactive Console) |
                              +-----------------------------+
                                             |
                              +-----------------------------+
                              |      Rust Kernel Core       |
                              | (no_std, Format, Commands)  |
                              +-----------------------------+
                                             |  (FFI Bridge)
                              +-----------------------------+
                              |    C HAL & Device Drivers   |
                              | (GDT, IDT, PIC, PIT, Serial)|
                              +-----------------------------+
                                             |
                              +-----------------------------+
                              |    Assembly Glue & MBR      |
                              | (boot.asm, kernel_entry.asm)|
                              +-----------------------------+
                                             |
                              +-----------------------------+
                              |     Bare-Metal Hardware     |
                              +-----------------------------+
````

### 1. Bootloader & Low-Level Assembly (`boot/`)

* **`boot/boot.asm`**: 512-byte MBR bootloader that loads the kernel using BIOS LBA Extended Read (`int 0x13, AH=0x42`) with a 64-sector chunk buffer and CHS fallback. Enables the Fast A20 Gate, loads a flat 32-bit GDT, and switches the CPU into 32-bit Protected Mode.
* **`boot/kernel_entry.asm`**: 32-bit mode entry point (`0x10000`), initializes the stack at `0x90000`, enables the FPU/SSE units through `CR0/CR4`, and provides assembly trampoline stubs for 32 CPU exception vectors (ISR 0–31) and 16 hardware IRQs (IRQ 0–15).

### 2. Hardware Abstraction Layer / HAL (`hal/`)

* **`hal/io.h` & `hal/io.c`**: x86 port I/O primitives (`inb`, `outb`, `inw`, `outw`, `cli`, `sti`, `hlt`).
* **`hal/vga.h` & `hal/vga.c`**: VGA text-mode 80x25 buffer driver (`0xB8000`) with foreground/background color management, automatic scrolling, and CRT controller hardware cursor updates (`0x3D4`/`0x3D5`).
* **`hal/gdt.h`, `hal/gdt.c`**: Global Descriptor Table with Kernel Code (`0x08`), Kernel Data (`0x10`), User Code (`0x18`), and User Data (`0x20`) segments.
* **`hal/idt.h`, `hal/idt.c`, `hal/isr.h`, `hal/isr.c`**: 256-gate Interrupt Descriptor Table, 8259 PIC remapping (Master IRQ 0..7 → 32..39, Slave IRQ 8..15 → 40..47), and CPU exception & IRQ dispatcher.
* **`hal/timer.h` & `hal/timer.c`**: PIT 8254 Channel 0 running at 100Hz (10ms per tick), with uptime tracking and delay/sleep functions.
* **`hal/keyboard.h` & `hal/keyboard.c`**: Interrupt-driven PS/2 keyboard driver (IRQ 1) with a circular ring buffer, Scancode Set 1 (US QWERTY) mapping, Shift modifier, CapsLock, and keypad support.
* **`hal/mouse.h` & `hal/mouse.c`**: Interrupt-driven PS/2 mouse driver (IRQ 12) with 3-byte streaming packet decoding, sign extension, and X/Y coordinate tracking.
* **`hal/serial.h` & `hal/serial.c`**: UART 16550 serial port driver (COM1 `0x3F8` @ 38400 baud) for kernel logging and debugging.

### 3. Rust Core & Shell Subsystem (`rust/`)

* **`rust/src/lib.rs`**: `#![no_std]` Rust entry point (`nyxara_rust_main`), boot ASCII banner, and a custom red-background panic handler for unhandled panics.
* **`rust/src/vga.rs`**: Safe VGA writer implementing `core::fmt::Write`, providing `print!`, `println!`, and `print_colored!` macros.
* **`rust/src/framebuffer.rs`**: TrueColor linear framebuffer engine, software text cursor, and graphical mouse arrow cursor overlay with background preservation.
* **`rust/src/mouse.rs`**: Safe Rust abstraction for reading mouse pointer position and button states.
* **`rust/src/serial.rs`**: Safe serial logger implementing `log!` and `logln!` macros.
* **`rust/src/shell.rs`**: Interactive line editor with the `nyxara> ` prompt, backspace handling, and command execution.
* **`rust/src/commands.rs`**: Built-in interactive shell commands.

---

## 💻 Shell Command List

| Command           | Description                                                                |
| ----------------- | -------------------------------------------------------------------------- |
| `help`            | Displays the available commands and usage information                      |
| `clear`           | Clears the screen and displays the Nyxara banner again                     |
| `about`           | Displays information about the hybrid C & Rust OS architecture             |
| `sysinfo`         | Displays CPU mode, stack pointer, interrupt status, and timer ticks        |
| `uptime`          | Displays the system uptime since boot                                      |
| `echo <text>`     | Prints the provided text back to the console                               |
| `color <fg> <bg>` | Dynamically changes the console colors (0..15)                             |
| `calc <a op b>`   | Integer arithmetic calculator (e.g. `calc 42 + 58`, `calc 100 * 5`)        |
| `mouse [test]`    | Displays PS/2 mouse driver status or enables visual pointer tracking mode  |
| `paint`           | Interactive GUI drawing canvas application using the framebuffer and mouse |
| `panic [message]` | Triggers a Rust Kernel Panic to demonstrate the crash handler              |
| `reboot`          | Performs a soft CPU reboot                                                 |

---

## 📁 Directory Structure

```text
Nyxara/
├── boot/
│   ├── boot.asm             # MBR Bootloader (16-bit real mode -> 32-bit protected mode)
│   └── kernel_entry.asm     # 32-bit Entry point, FPU/SSE setup & ISR stubs
├── hal/
│   ├── types.h              # Freestanding primitive typedefs & memory prototypes
│   ├── string.c             # Freestanding memcpy, memset, memcmp, bcmp, strlen implementations
│   ├── io.h / io.c          # Port I/O wrappers (inb, outb, inw, outw, cli, sti, hlt)
│   ├── vga.h / vga.c        # VGA Text Console 80x25 driver
│   ├── fb.h / fb.c          # VBE Linear Framebuffer detection & initialization
│   ├── gdt.h / gdt.c        # Global Descriptor Table (GDT)
│   ├── idt.h / idt.c        # Interrupt Descriptor Table (IDT) & PIC 8259 Remapping
│   ├── isr.h / isr.c        # Interrupt Service Routines & Exception handlers
│   ├── timer.h / timer.c    # PIT (Programmable Interval Timer) 100Hz
│   ├── keyboard.h / keyboard.c # PS/2 Keyboard driver with ring buffer
│   ├── mouse.h / mouse.c    # PS/2 Mouse driver (IRQ 12) & packet stream decoder
│   ├── serial.h / serial.c  # UART 16550 Serial COM1 driver
│   ├── pci.h / pci.c        # PCI Bus scanner & device discovery
│   ├── rtl8139.h / rtl8139.c# RTL8139 Fast Ethernet NIC driver
│   ├── rtc.h / rtc.c        # CMOS / RTC Real-Time Clock driver
│   └── hal.h                # Unified HAL Master Header
├── kernel/
│   └── kmain.c              # C Kernel initialization & Rust bridge
├── rust/
│   ├── Cargo.toml           # Rust package configuration
│   └── src/
│       ├── lib.rs           # Rust kernel entry, panic handler, banner
│       ├── vga.rs           # Safe VGA writer & print! macros
│       ├── framebuffer.rs   # LFB TrueColor renderer & mouse cursor sprite
│       ├── mouse.rs         # Safe Rust PS/2 mouse interface
│       ├── serial.rs        # Safe Serial logger & log! macros
│       ├── shell.rs         # Interactive line-buffered shell
│       ├── editor.rs        # Full-screen text editor
│       ├── net/             # Network stack (TCP, UDP, IPv4, DHCP, DNS, HTTP)
│       └── commands.rs      # Command interpreter engine
├── linker.ld                # Linker script (loaded at 0x10000)
├── Makefile                 # Modular build system
└── README.md                # Project documentation
```

---

## 🛠️ Prerequisites & Toolchain Setup

### On Linux (Arch Linux / Debian / Ubuntu / Fedora)

* **NASM** (`nasm`)
* **GCC** (`gcc`)
* **GNU Binutils** (`ld`, `objcopy`)
* **Rust Toolchain** (`rustc`, `cargo` with the `i686-unknown-linux-gnu` target)
* **QEMU** (`qemu-system-i386`)
* **Make** (`make`)

```bash
# Arch Linux
sudo pacman -S nasm gcc binutils qemu-system-x86 make rust

# Debian / Ubuntu
sudo apt install nasm gcc-multilib binutils qemu-system-x86 make rustc cargo

# Rust 32-bit x86 target
rustup target add i686-unknown-linux-gnu
```

---

## 🚀 Building & Running

### 1. Build the OS Image

```bash
make clean
make
```

This command compiles the NASM bootloader, C HAL, Rust static library, performs linking through `ld`, and generates the `nyxara.img` disk image.

### 2. Run with QEMU

```bash
# Run with the QEMU graphical display
make run

# Run with serial output connected to terminal stdio
make run-serial

# Run in Curses text console mode
make run-curses
```

### 3. Debugging with GDB

```bash
make debug
```

QEMU will wait for a GDB connection on port `localhost:1234`. In another terminal, run:

```bash
gdb -ex "target remote localhost:1234" -ex "symbol-file build/kernel.elf"
```

---

## 📜 License

Free to use, modify, and develop for educational purposes and operating system development experimentation.

```
```
