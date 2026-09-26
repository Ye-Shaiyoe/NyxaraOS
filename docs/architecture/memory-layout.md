# Memory Layout and Addressing

Nyxara OS operates in x86 32-bit Protected Mode using both flat segmentation and two-level page translation (paging). This document details the physical memory organization, virtual address space mapping, and stack allocations.

## Physical Memory Map

Below is the layout of physical RAM from `0x00000000` upwards:

```
+-----------------------------------+ 0x00000000
| Real Mode IVT & BIOS Data Area    | (1 KB IVT + 256 B BDA)
+-----------------------------------+ 0x00000500
| Free Low Memory Scratch Space     |
+-----------------------------------+ 0x00007C00
| MBR Bootloader (512 Bytes)        | Loaded by BIOS
+-----------------------------------+ 0x00007E00
| VBE Mode Info Scratch Buffer      | 256 Bytes buffer for int 0x10, ax=0x4F01
+-----------------------------------+ 0x00009000
| Boot Information Block (boot_info)| Resolution, LFB address, BPP, Magic
+-----------------------------------+ 0x00010000 (64 KB)
| Kernel Binary Image (ELF Sections)|
|   .text (Code)                    |
|   .rodata (Read-Only Data)        |
|   .data (Initialized Data)        |
|   .bss (Zero-Initialized Data)    |
|   kernel_end symbol               |
+-----------------------------------+ Dynamic (~0x70000)
| Dynamic Kernel Heap (4 MB)        | Managed by `heap::KernelAllocator`
+-----------------------------------+ ~0x00100000 (1 MB Mark)
| Extended Memory Control Structures|
|   0x00100000: Page Directory (4KB)|
|   0x00101000: LFB Page Table (4KB)|
|   0x00110000: Identity Tables     | (16 Page Tables = 64 KB, maps 64 MB)
|   0x00130000..0x00140000: Stack   | 64 KB Kernel Call Stack (Top: 0x140000)
+-----------------------------------+ 0x00140000
| Physical Memory Page Pool         | Managed by PMM Bitmap (up to 128 MB)
+-----------------------------------+ Top of Physical RAM
```

## Hardware Memory Regions

- **`0x000A0000 - 0x000BFFFF`**: Legacy Video Memory (VGA text buffer at `0x000B8000`).
- **`0x000E0000 - 0x000FFFFF`**: System BIOS and Extended BIOS Data Area (EBDA).
- **PCI Framebuffer Space (BAR)**: Linear Framebuffer base pointer dynamically queried via VBE (`PhysBasePtr` around `0xFD000000` or `0xE0000000`).

## Virtual Memory Organization

When paging is enabled (`CR0.PG = 1` and `CR3 = 0x00100000`), the 4 GB virtual address space is partitioned as follows:

| Virtual Address Range | Size | Description | Attributes |
|---|---|---|---|
| `0x00000000 - 0x03FFFFFF` | 64 MB | Identity-mapped physical memory (Kernel code, data, heap, low hardware) | Supervisor, Read/Write, Present |
| `0x04000000 - 0x04001FFF` | 8 KB | Ring 3 demo code (`0x04000000`) and user stack (`0x04001000`) | User; stack writable |
| `0x04002000 - 0xBFFFFFFF` | Remainder | Not generally mapped; intended for future user processes | Unmapped by default |
| `0xC0000000 - 0xC1000000` | 16 MB | Dedicated Demand Paging Region (Allocated on first access via Page Fault) | Supervisor, Read/Write, Dynamic |
| `PhysBasePtr - PhysBasePtr+N` | Dynamic | VBE Linear Framebuffer video memory mapped 1:1 | Supervisor, Read/Write, Write-through |

## Global Descriptor Table (GDT) Segmentation

The GDT provides flat 4 GB addressing across all segments, ensuring segment limits do not restrict virtual paging. The two user selectors exist, but the current demo shares the kernel's page directory; these mappings are not separate per-process address spaces. See [Processes, Scheduling, and Ring 3](../kernel/processes.md).

| Selector | Name | Base | Limit | Priv | Type / Flags |
|---|---|---|---|---|---|
| `0x00` | Null Descriptor | `0x00000000` | `0x00000` | - | - |
| `0x08` | Kernel Code Segment | `0x00000000` | `0xFFFFF` | Ring 0 | Executable, Readable, 32-bit, 4KB granularity |
| `0x10` | Kernel Data Segment | `0x00000000` | `0xFFFFF` | Ring 0 | Read/Write, 32-bit, 4KB granularity |
| `0x18` | User Code Segment | `0x00000000` | `0xFFFFF` | Ring 3 | Executable, Readable, 32-bit, 4KB granularity |
| `0x20` | User Data Segment | `0x00000000` | `0xFFFFF` | Ring 3 | Read/Write, 32-bit, 4KB granularity |
| `0x28` | Task State Segment (TSS) | Dynamic | sizeof(TSS) - 1 | Ring 0 | Present, available 32-bit TSS descriptor |

## Memory Protection and Page Flags

Every 4 KB page table entry contains x86 standard permission bits:

```
31                                    12 11   9  8 7 6 5 4 3 2 1 0
+---------------------------------------+------+--+-+-+-+-+-+-+-+-+
|       Physical Frame Address          | Avail|G |S|D|A|C|W|U|W|P|
+---------------------------------------+------+--+-+-+-+-+-+-+-+-+
```

- **Bit 0 (`P`)**: Page Present (1 = in physical memory, 0 = triggers ISR 14 Page Fault).
- **Bit 1 (`W`)**: Read/Write (1 = writable, 0 = read-only).
- **Bit 2 (`U`)**: User/Supervisor (1 = accessible from Ring 3, 0 = Ring 0 only).
- **Bit 3 (`W`)**: Write-through caching policy.
- **Bit 4 (`C`)**: Cache disable flag (useful for memory-mapped I/O).
- **Bit 5 (`A`)**: Accessed bit set by CPU hardware on page read/write.
- **Bit 6 (`D`)**: Dirty bit set by CPU on write.
