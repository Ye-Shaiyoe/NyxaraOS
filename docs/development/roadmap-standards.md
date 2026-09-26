# Standards, Workspace Roles & Roadmap

This document defines the engineering standards, directory responsibilities, and future architectural milestones for Nyxara OS.

## Strict Code Style Standards

Nyxara enforces a clean, minimalistic, and maintainable codebase as specified in `CLAUDE.md`:

1. **No AI Slop / Over-commenting**
   - Redundant or obvious comments are prohibited (e.g. repeating function names in docstrings or commenting lines like `// initialize x`).
   - Comments must be reserved exclusively for non-obvious hardware behaviors, register bitmasks, or complex low-level algorithms.

2. **No Heavy ASCII Separators**
   - Decorative dividers such as `// ==========================`, `/* ----------------- */`, or `##########` are strictly forbidden in new code.
   - Clean, standard spacing and concise markdown headers must be used instead.

3. **Code Conciseness & Language Policy**
   - **Rust**: `#![no_std]`, `#![no_main]`. Encapsulate hardware access in safe abstractions with explicit, minimal `unsafe` blocks.
   - **C**: Freestanding compilation (`-ffreestanding -nostdlib`). Use explicit fixed-width integers (`uint32_t`, `size_t`) from `hal/types.h`.
   - **Assembly**: NASM syntax with minimal boilerplate.

---

## Directory Roles & Workspace Architecture

The repository is divided into focused directories to separate responsibilities:

| Directory | Purpose & Status |
|---|---|
| **`boot/`** | Active. 16-bit MBR loader (`boot.asm`) and 32-bit protected mode entry (`kernel_entry.asm`). |
| **`hal/`** | Active. Hardware Abstraction Layer in C (GDT, IDT, PIC, PIT, UART, PCI, RTL8139, VGA/LFB, PS/2). |
| **`kernel/`** | Active. Kernel initialization (`kmain.c`) and C-to-Rust calling trampolines. |
| **`rust/`** | Active. Core Rust kernel (PMM, Heap, VMM Paging, Syscalls, VFS, Network Stack, Shell, Editor). |
| **`drivers/`** | Planned. Modular driver plugins built on top of HAL (sound, USB, AHCI). |
| **`libc/`** | Planned. Freestanding C runtime library and syscall wrappers for Ring 3 user programs. |
| **`userland/`** | Planned. Standalone user applications and services executing outside supervisor mode. |
| **`fs/`** | Planned. Persistent block storage filesystem drivers (ext2, FAT32). |
| **`include/`** | Planned. Shared header files exposed to drivers and userland. |
| **`tests/`** | Active, partial. Host-run line editor and VFS tests; automated integration and emulator validation remain future work. |
| **`tools/`** | Planned. Disk image packaging, asset generators, and development utilities. |
| **`config/`** | Target architecture and build configuration files. |

---

## Architectural Roadmap

### Phase 1: Preemptive Multitasking & Scheduler
**Status: Prototype implemented.** `rust/src/process.rs` has a fixed-size task table, Ready/Running/Blocked states, a five-tick round-robin quantum, sleep/wakeup, and IRQ-frame switching. It starts demo kernel tasks. Per-process page directories, Zombie state, resource cleanup, and production worker threads remain future work. See [Processes, Scheduling, and Ring 3](../kernel/processes.md).

### Phase 2: Ring 3 Userland Process Isolation
**Status: Privilege-transition demo implemented; process isolation incomplete.** The TSS, user selectors, DPL 3 syscall gate, and one Ring 3 demo task are present. The TSS still uses a fixed initial kernel stack, tasks share one page directory, and syscall pointers are not validated. Per-task `esp0`, isolated address spaces, and a complete syscall ABI remain future work.

### Phase 3: ELF Binary Executable Loader
- Parse 32-bit ELF headers and program headers (`PT_LOAD`).
- Map ELF segments into user address space with appropriate page permissions (Executable vs Writable).
- Jump to the ELF entry point in Ring 3 via an `iret` frame.

### Phase 4: Persistent Block Storage & Filesystems
- Implement ATA/IDE PIO and AHCI disk controllers.
- Develop an ext2 and FAT32 filesystem driver in Rust, mounting onto the existing VFS node tree.
- Persist shell user files across reboots and QEMU sessions.
