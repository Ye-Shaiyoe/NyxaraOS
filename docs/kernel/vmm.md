# Virtual Memory Manager (VMM) & x86 Paging

The Virtual Memory Manager (`rust/src/vmm.rs`) implements standard two-level x86 page translation, memory protection, linear framebuffer mapping, and demand paging triggered via CPU Exception 14 (Page Fault).

## Two-Level Page Translation Architecture

x86 32-bit hardware paging maps a 32-bit linear virtual address into a 32-bit physical address through a Page Directory and Page Tables:

```
Virtual Address (32-bit):
+--------------------+--------------------+--------------------+
| Directory (10 bit) | Table (10 bit)     | Offset (12 bit)    |
| Bits 31..22        | Bits 21..12        | Bits 11..0         |
+--------------------+--------------------+--------------------+
         |                    |                    |
         v                    v                    v
  [Page Directory] ---> [Page Table] -------> [Physical 4KB Frame]
  (CR3 Register)
```

## Physical Placement in Extended DRAM

All core paging structures reside in safe extended physical memory above 1 MB:
- **`0x00100000` (`PAGE_DIR_PHYS`)**: Page Directory (4 KB, 1,024 entries).
- **`0x00101000` (`LFB_TABLE_PHYS`)**: Linear Framebuffer Page Table (4 KB).
- **`0x00110000` (`IDENT_TABLES_PHYS`)**: 16 Identity Page Tables (64 KB total).

## Identity Mapping (0 to 64 MB)

To ensure the kernel code, HAL data, stacks, heap, and memory-mapped BIOS regions execute transparently without address translation overhead:
- 16 Page Tables are populated to identity map `0x00000000` through `0x03FFFFFF` (64 MB).
- Every entry has flags: `PAGE_PRESENT | PAGE_WRITABLE` (`0x03`).
- Page Directory entries `0` through `15` point directly to these identity tables.

## Framebuffer Memory-Mapped I/O

The VBE Linear Framebuffer is mapped to ensure video memory writes bypass cache buffering:
- Page Directory index computed as `(lfb_phys >> 22) & 0x3FF`.
- Entries flagged with `PAGE_PRESENT | PAGE_WRITABLE | PAGE_NOCACHE` (`0x13`).

## Demand Paging Subsystem

Nyxara supports supervisor-only demand paging within a reserved 16 MB virtual address window:
- **Range**: `0xC0000000` to `0xC1000000`.
- When a Ring 0 instruction reads or writes to an unmapped page in this range, the page table initially marks the page as not present (`P = 0`). The handler allocates a frame and maps it writable for the kernel.
- The CPU immediately triggers an **ISR 14 Page Fault**.
- A Ring 3 fault does not receive a user mapping from this handler; faults outside the supported supervisor demand-page case are fatal.

### Page Fault Handler (`page_fault_handler`)

The handler reads the faulting address from `CR2` and examines the CPU error code. It handles a non-present page fault only when the address is within the demand-paging range: it allocates and clears a frame, maps it supervisor-writable, invalidates the TLB entry, and returns so the CPU retries the instruction. Other page faults produce a diagnostic and halt the kernel.

## Activating Paging

During `vmm::init()`:
1. Load Page Directory address into `CR3`:
   ```rust
   core::arch::asm!("mov cr3, {0}", in(reg) PAGE_DIR_PHYS);
   ```
2. Enable Write-Protect (`WP`, bit 16) and Paging (`PG`, bit 31) in `CR0`:
   ```rust
   let mut cr0: usize;
   core::arch::asm!("mov {0}, cr0", out(reg) cr0);
   cr0 |= (1 << 31) | (1 << 16);
   core::arch::asm!("mov cr0, {0}", in(reg) cr0);
   ```

## Automated Verification (`test_vmm`)

At boot time, `vmm::test_vmm()` verifies identity mappings, writes `0x5A5A1234` to the previously unmapped address `0xC0002000`, checks the value and mapping, then tests manual mapping and unmapping at `0xD0001000`.

The Ring 3 demo adds user mappings at `0x04000000` and `0x04001000` to the existing page directory. This is a single shared address space, not per-process isolation. See [Processes, Scheduling, and Ring 3](processes.md).
