# Nyxara OS Documentation

Welcome to the official documentation for **Nyxara OS**, a hybrid Unix-like operating system built from scratch for x86 (32-bit Protected Mode). Nyxara combines low-level hardware control using C and Assembly with the safety, modern abstractions, and expressive design of freestanding Rust (`#![no_std]`).

## Documentation Index

### 1. [System Architecture](architecture/overview.md)
Foundational design principles, privilege boundaries, memory maps, and system life-cycle.
- [Architecture Overview](architecture/overview.md) - Hybrid model, component breakdown, and design philosophy.
- [Memory Layout](architecture/memory-layout.md) - Physical and virtual address spaces, segment descriptors, and paging structures.
- [Execution Flow](architecture/execution-flow.md) - End-to-end boot sequence from MBR reset vector to the interactive shell.

### 2. [Bootloader Subsystem](bootloader/mbr.md)
Low-level firmware hand-off, disk reading, mode switching, and CPU state initialization.
- [Master Boot Record (MBR)](bootloader/mbr.md) - 16-bit real mode, LBA chunk loader, A20 gate, VBE modes, and protected mode switch.
- [Kernel Entry & Stubs](bootloader/kernel-entry.md) - 32-bit protected mode entry, stack allocation, SSE/FPU initialization, and ISR/IRQ assembly trampolines.

### 3. [Hardware Abstraction Layer (HAL)](hal/overview.md)
C-based driver ecosystem interfacing directly with x86 motherboard logic and peripherals.
- [HAL Overview & Freestanding C](hal/overview.md) - Types, memory primitives (`memcpy`, `memset`), and hardware interface contracts.
- [GDT, IDT & Interrupt Dispatcher](hal/gdt-idt.md) - Segment descriptors, PIC 8259 remap, and the 256-entry interrupt descriptor table.
- [Port I/O & Serial COM1](hal/io-serial.md) - Port I/O instructions (`inb`, `outb`) and UART 16550 serial debugging output.
- [Timers & Real-Time Clock](hal/timers-interrupts.md) - PIT 8254 100Hz system tick and CMOS/RTC calendar clock driver.
- [Input Drivers](hal/input.md) - PS/2 Keyboard scancode processor and PS/2 Mouse streaming packet decoder.
- [Video Drivers](hal/video.md) - VGA 80x25 text console buffer and VBE linear framebuffer (LFB) mode.
- [PCI Bus & Network Interface](hal/pci-network.md) - PCI configuration space scanner and Realtek RTL8139 Fast Ethernet driver.

### 4. [Kernel Subsystems](kernel/overview.md)
Core operating system logic implemented in safe, idiomatic Rust with explicit FFI bridges.
- [Kernel Core Overview](kernel/overview.md) - C `kmain` handoff, Rust entry point, and custom panic handler.
- [Physical Memory Manager (PMM)](kernel/pmm.md) - Frame allocator using a bitmap, tracking physical memory pages (4KB).
- [Virtual Memory Manager (VMM)](kernel/vmm.md) - Two-level x86 paging, identity mapping, demand paging, and ISR 14 page fault handling.
- [Kernel Heap Allocator](kernel/heap.md) - Global allocator implementation enabling dynamic allocations (`alloc::vec`, `alloc::string`).
- [Unix-like System Calls](kernel/syscalls.md) - Software interrupt `int 0x80`, ABI register conventions, and current implementation limits.
- [Processes, Scheduler & Ring 3](kernel/processes.md) - Current task scheduler, TSS privilege transition, Ring 3 demo, and isolation limits.
- [Virtual File System (VFS)](kernel/vfs.md) - Inode abstraction, RamFS in-memory filesystem, and device nodes (`/dev/tty`).

### 5. [Network Stack](networking/stack-overview.md)
Modular TCP/IP networking implementation from link layer to application services.
- [Stack Architecture Overview](networking/stack-overview.md) - Layered packet processing model and hardware RX/TX ring buffers.
- [Protocols (L2–L4)](networking/protocols.md) - Ethernet frames, ARP cache, IPv4 routing, ICMP Ping, UDP, and TCP state machine.
- [Network Services (L7)](networking/services.md) - DHCP client (DORA), DNS resolver, HTTP client (`curl`), embedded HTTP web server (`httpd`), and Netcat (`nc`).

### 6. [User Interface & Interactive Tools](user-interface/shell.md)
Interactive command console, terminal utilities, text editor, and graphics engine.
- [Interactive Shell](user-interface/shell.md) - Line editor, prompt rendering, color coding, and command dispatcher.
- [Shell Commands Reference](user-interface/commands-reference.md) - Comprehensive command-line reference for all built-in commands.
- [Text Editor (`mway`)](user-interface/editor-mway.md) - Full-screen text editor with keybindings, scrolling, and VFS file persistence.
- [Graphics & Canvas (`paint`)](user-interface/graphics-paint.md) - Framebuffer engine, software mouse cursor overlay, 8x16 font rendering, and interactive painting app.

### 7. [Development & Maintenance](development/build-system.md)
Engineering workflow, compilation targets, debugging setups, and development standards.
- [Build System & Linker](development/build-system.md) - Multi-language Makefile targets, binary compilation pipeline, and `linker.ld` script.
- [Prerequisites & Toolchain](development/prerequisites.md) - Cross-compilation tools (NASM, GCC, Rust target `i686-unknown-linux-gnu`, QEMU).
- [Debugging Guide](development/debugging.md) - GDB remote targets, serial monitor capture, and diagnosing kernel panics.
- [Testing Guide](development/testing.md) - Host tests, kernel self-tests, and QEMU smoke-test expectations.
- [Standards & Roadmap](development/roadmap-standards.md) - Code quality rules, directory responsibilities, and future milestones.
