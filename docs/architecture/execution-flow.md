# System Execution and Boot Flow

This document details the lifecycle of Nyxara OS from computer power-on through firmware handoff, HAL bootstrap, Rust subsystem initialization, to the interactive shell prompt.

## High-Level Lifecycle Diagram

```
+-------------------------------------------------------------------+
| 1. BIOS Phase                                                     |
|    Power-On Self-Test (POST) -> Read Sector 0 (MBR) into 0x7C00   |
+-------------------------------------------------------------------+
                                  |
+-------------------------------------------------------------------+
| 2. Real Mode Bootloader (boot/boot.asm, 16-bit)                   |
|    - Reset Segment Registers & Setup SP = 0x7C00                  |
|    - Read Kernel Sectors via LBA Extended Read (int 0x13, AH=0x42)|
|    - Initialize VBE Graphics Mode (800x600 / 1024x768 LFB)        |
|    - Enable Fast A20 Gate via Port 0x92                           |
|    - Load Temporary GDT & Switch CPU to Protected Mode (CR0.PE)   |
|    - Far Jump to 32-bit entry (0x10000)                           |
+-------------------------------------------------------------------+
                                  |
+-------------------------------------------------------------------+
| 3. Assembly Glue (boot/kernel_entry.asm, 32-bit)                  |
|    - Setup Data Segments (0x10) & Kernel Stack Top (0x140000)     |
|    - Zero-out the .bss section (rep stosb)                        |
|    - Enable FPU & SSE Instructions (CR0.MP, CR4.OSFXSR)           |
|    - Push pointer to BootInfo (0x9000) & invoke C kmain           |
+-------------------------------------------------------------------+
                                  |
+-------------------------------------------------------------------+
| 4. C Hardware Abstraction Layer (kernel/kmain.c, hal/)            |
|    - gdt_init()       : Flat GDT + TSS descriptor                 |
|    - idt_init()       : 256 gates, PIC 8259 remap (32..47)        |
|    - fb_init()        : Detect LFB or fallback to VGA 80x25       |
|    - serial_init()    : UART 16550 COM1 @ 38400 baud              |
|    - timer_init(100)  : PIT 8254 Channel 0 (100 Hz, 10ms tick)    |
|    - keyboard_init()  : PS/2 Keyboard IRQ 1 with ring buffer      |
|    - mouse_init()     : PS/2 Mouse IRQ 12 streaming packet parser |
|    - pci_init()       : Enumerate PCI bus devices                 |
|    - rtl8139_init()   : RTL8139 Fast Ethernet NIC initialization  |
|    - rtc_init()       : Read Real-Time Clock from CMOS registers  |
|    - sti()            : Enable hardware interrupts                |
|    - Align stack to 16 bytes and call nyxara_rust_main            |
+-------------------------------------------------------------------+
                                  |
+-------------------------------------------------------------------+
| 5. Rust Kernel Core (rust/src/lib.rs)                             |
|    - pmm::init()      : Bitmap Physical Frame Allocator (4KB)     |
|    - heap::init()     : 4 MB Dynamic Kernel Heap for `alloc` crate|
|    - vmm::init()      : 2-Level Paging, Page Dir (CR3), CR0.PG    |
|    - syscall::init()  : Register ISR 128 for `int 0x80` traps     |
|    - vfs::init()      : Initialize RamFS with initial files       |
|    - net::init()      : Initialize network configuration & ARP    |
|    - vmm::test_vmm()  : Self-test identity and demand paging      |
|    - process::init()  : Start scheduler and Ring 3 demo task      |
+-------------------------------------------------------------------+
                                  |
+-------------------------------------------------------------------+
| 6. Interactive Subsystem (rust/src/shell.rs)                      |
|    - Display ASCII Art Banner & Welcome Information               |
|    - Enter Read-Eval-Print Loop (REPL)                            |
|    - Listen for PS/2 Scancodes & Keyboard Inputs                  |
|    - Execute Built-in Shell Commands & Dispatch Tasks             |
+-------------------------------------------------------------------+
```

## Step-by-Step Transition Details

### 1. BIOS to MBR Handoff
- The system BIOS completes hardware testing and reads the first physical sector (512 bytes) of the bootable drive to physical address `0x0000:0x7C00`.
- The BIOS passes the boot drive index in CPU register `DL` (typically `0x80` for hard drive/disk image).
- Execution begins at `boot/boot.asm` with instruction pointer `0x7C00`.

### 2. Loading the Kernel via BIOS LBA
- Floppy/disk sectors cannot be read in a single BIOS call if the image exceeds track boundaries. Nyxara's MBR uses an LBA chunk loop:
  - Calls BIOS Extended Read function `int 0x13, AH=0x42` using a Disk Address Packet (DAP).
  - Reads in chunks of 64 sectors (32 KB per chunk) to avoid 64 KB segment boundary overflows.
  - Progressively increments memory segment pointers (`dap_segment += num_sectors * 32`) until all kernel sectors are loaded into memory starting at physical `0x00010000`.

### 3. Transition to 32-bit Protected Mode
- The A20 gate is enabled via port `0x92` to avoid 1 MB memory wrap-around.
- A flat Global Descriptor Table is loaded using `lgdt`.
- Bit 0 (`PE` - Protection Enable) of `CR0` is set to 1.
- A far jump (`jmp 0x08:protected_mode_entry`) clears the 16-bit prefetch queue and serializes pipeline execution in 32-bit mode.

### 4. Stack and Environment Initialization
- Segment registers (`DS`, `ES`, `FS`, `GS`, `SS`) are assigned selector `0x10` (Kernel Data).
- The assembly glue zero-initializes the BSS segment to satisfy standard C and Rust language expectations.
- FPU control registers and SSE support are configured on `CR0` and `CR4`.

### 5. Transition to Rust Core
- C `kmain` executes all necessary hardware initialization while interrupts are cleanly managed.
- After all drivers are verified over COM1 serial logging, `kmain` aligns the stack pointer to a 16-byte boundary complying with the System V i386 ABI before calling `nyxara_rust_main`.
- Rust takes complete control over system memory allocation, paging tables, VFS state, and console interaction.
- After VMM self-tests, `process::init()` starts the round-robin scheduler and creates demo tasks, including a Ring 3 task. This demonstrates a privilege transition and `int 0x80`, but does not provide per-process memory isolation. See [Processes, Scheduling, and Ring 3](../kernel/processes.md).
