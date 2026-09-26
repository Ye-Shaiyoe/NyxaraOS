# System Architecture Overview

Nyxara is a hybrid 32-bit x86 Unix-like operating system designed around clean separation of concerns: low-level hardware abstraction implemented in freestanding C and NASM Assembly, coupled with safe kernel primitives and shell systems written in freestanding Rust (`#![no_std]`).

```
+-------------------------------------------------------------+
|                        Nyxara Shell                         |
|     (Rust Interactive Console, Line Editor, Built-in Tools) |
+-------------------------------------------------------------+
                               |
+-------------------------------------------------------------+
|                      Rust Kernel Core                       |
|   (PMM, VMM Paging, Heap Allocator, VFS, Network Stack,     |
|              System Calls & Command Dispatcher)             |
+-------------------------------------------------------------+
                               |  FFI Bridge (C calling convention)
+-------------------------------------------------------------+
|              Hardware Abstraction Layer (HAL)               |
|      (GDT, IDT, PIC 8259, PIT Timer, Serial COM1, PCI,      |
|              RTL8139 NIC, PS/2 Input, VGA/LFB)              |
+-------------------------------------------------------------+
                               |
+-------------------------------------------------------------+
|                 Assembly Glue & Bootloader                  |
|          (boot/boot.asm MBR, boot/kernel_entry.asm)         |
+-------------------------------------------------------------+
                               |
+-------------------------------------------------------------+
|                     x86 Bare Metal / QEMU                   |
+-------------------------------------------------------------+
```

## Hybrid Design Philosophy

1. **Hardware Abstraction Layer in C (`hal/`)**
   - Direct hardware manipulation (Port I/O, bitwise register masks, descriptor structures).
   - Minimal C runtime without standard library (`-ffreestanding -nostdlib`).
   - Self-contained string manipulation (`memcpy`, `memset`, `strlen`) matching standard C ABI.

2. **Kernel Logic & Subsystems in Rust (`rust/src/`)**
   - High safety guarantees: zero-cost abstractions, pattern matching, bounds-checking options, and strict ownership for internal states.
   - Modern standard abstractions via `alloc` (dynamic vectors, strings, and boxed structures).
   - Rich networking implementation (Ethernet, ARP, IPv4, ICMP, UDP, TCP, DHCP, DNS, HTTP) without relying on libc.

3. **FFI Bridge (`extern "C"`)**
   - Hardware drivers expose C ABI functions (`timer_get_ticks`, `serial_puts`, `keyboard_getchar`, `rtl8139_send_packet`).
   - Rust kernel core consumes these functions directly via `extern "C"` declarations.
   - Assembly entry points bridge directly into C (`kmain`), which in turn aligns the stack and enters Rust (`nyxara_rust_main`).

## Privilege Rings and CPU Mode

| CPU Mode / Ring | Component | Role |
|---|---|---|
| **Ring 0 (Supervisor)** | Assembly Glue | CPU initialization, GDT/IDT loading, CR0/CR4 control, interrupt vectoring |
| **Ring 0 (Supervisor)** | HAL Drivers (C) | Direct device programming (I/O ports, PIC, PIT, UART, PCI, NIC) |
| **Ring 0 (Supervisor)** | Kernel Core (Rust) | Paging tables (CR3), frame allocation, VFS, networking, line editor |
| **Ring 3 (Prototype)** | User demo task | Enters through `iret` and calls the kernel through `int 0x80`; per-process isolation remains future work |

## Key Technical Specifications

- **Target Architecture**: x86 32-bit Protected Mode (`i686-unknown-linux-gnu` target).
- **Physical Memory Support**: Up to 128 MB physical DRAM managed via 4 KB bitmap page frames.
- **Virtual Memory**: 2-level x86 paging (Page Directory and Page Tables), with 64 MB kernel identity mapping and dedicated demand-paging ranges (`0xC0000000..0xC1000000`).
- **Interrupts**: PIC 8259 remap (IRQ 0-15 to vectors 32-47), 32 CPU exception vectors, plus `int 0x80` trap gate for POSIX syscalls.
- **Display Modes**: VGA text mode 80x25 (`0xB8000`) and VBE Linear Framebuffer mode (800x600x32 or 1024x768x32).
- **Network Hardware**: Realtek RTL8139 PCI Fast Ethernet (10/100 Mbps).
