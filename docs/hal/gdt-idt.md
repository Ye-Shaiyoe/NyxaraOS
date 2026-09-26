# Global Descriptor Table (GDT) & Interrupt Management

Nyxara OS establishes a standard flat memory model via the Global Descriptor Table (GDT) and configures complete CPU exception and hardware interrupt routing via the Interrupt Descriptor Table (IDT) and the 8259 Programmable Interrupt Controller (PIC).

## Global Descriptor Table (GDT)

The GDT configuration resides in `hal/gdt.c` and `hal/gdt.h`. It defines 6 segment descriptors:

| Index | Selector | Description | Base | Limit | Access Byte | Flags |
|---|---|---|---|---|---|---|
| 0 | `0x00` | Null Descriptor | `0x00000000` | `0x00000000` | `0x00` | `0x00` |
| 1 | `0x08` | Kernel Code (Ring 0) | `0x00000000` | `0xFFFFFFFF` | `0x9A` (Present, Ring 0, Code, Exec/Read) | `0xCF` (4KB gran, 32-bit) |
| 2 | `0x10` | Kernel Data (Ring 0) | `0x00000000` | `0xFFFFFFFF` | `0x92` (Present, Ring 0, Data, Read/Write) | `0xCF` (4KB gran, 32-bit) |
| 3 | `0x18` | User Code (Ring 3) | `0x00000000` | `0xFFFFFFFF` | `0xFA` (Present, Ring 3, Code, Exec/Read) | `0xCF` (4KB gran, 32-bit) |
| 4 | `0x20` | User Data (Ring 3) | `0x00000000` | `0xFFFFFFFF` | `0xF2` (Present, Ring 3, Data, Read/Write) | `0xCF` (4KB gran, 32-bit) |
| 5 | `0x28` | Task State Segment (TSS) | `&tss_entry` | `sizeof(tss) - 1` | `0x89` (Present, Ring 0, available 32-bit TSS) | `0x00` (Byte gran) |

### Task State Segment (TSS)
The TSS holds the kernel stack pointer (`esp0` and `ss0 = 0x10`) required for hardware privilege transitions. `gdt_init()` currently initializes `esp0` to `0x00140000`; the scheduler does not yet update it to a per-task kernel stack. See [Processes, Scheduling, and Ring 3](../kernel/processes.md) for the current prototype and its limits.

## 8259 PIC Remapping

By default, legacy IBM PC BIOS assigns IRQ 0–7 to interrupt vectors 8–15. In Protected Mode, vectors 8–15 conflict with CPU hardware exceptions (e.g., Vector 8 is Double Fault, Vector 13 is General Protection Fault).

`idt_init()` remaps the 8259 Master and Slave PICs:
- **Master PIC**: Remapped from `0x08..0x0F` to vectors **32..39** (`0x20..0x27`).
- **Slave PIC**: Remapped from `0x70..0x77` to vectors **40..47** (`0x28..0x2F`).

```c
// ICW1: Start initialization in cascade mode
outb(0x20, 0x11);
outb(0xA0, 0x11);

// ICW2: Vector offsets
outb(0x21, 0x20); // Master offset 32
outb(0xA1, 0x28); // Slave offset 40

// ICW3: Cascading wiring (Master IRQ 2 connected to Slave)
outb(0x21, 0x04);
outb(0xA1, 0x02);

// ICW4: 8086/88 mode
outb(0x21, 0x01);
outb(0xA1, 0x01);
```

## Interrupt Descriptor Table (IDT)

The IDT consists of 256 8-byte descriptors mapping CPU vectors to assembly handlers:

```c
typedef struct {
    uint16_t base_low;   // Lower 16 bits of handler address
    uint16_t selector;   // Kernel code segment selector (0x08)
    uint8_t  always0;    // Reserved byte (always 0)
    uint8_t  flags;      // Type and attributes (P, DPL, Gate Type)
    uint16_t base_high;  // Upper 16 bits of handler address
} __attribute__((packed)) idt_entry_t;
```

### Gate Configuration
- **Vectors 0–31 (CPU Exceptions)**: Installed with flag `0x8E` (Present, Ring 0, 32-bit Interrupt Gate).
- **Vectors 32–47 (Hardware IRQs)**: Installed with flag `0x8E`.
- **Vector 128 (`0x80`, POSIX Syscall)**: Installed with flag `0xEE` (Present, Ring 3 DPL, 32-bit Trap Gate), enabling userland applications to trigger system calls without privilege violations.

## Interrupt Service Routines (ISR & IRQ)

The C dispatcher in `hal/isr.c` handles exceptions and hardware interrupts:

```c
typedef struct {
    uint32_t ds;
    uint32_t edi, esi, ebp, esp, ebx, edx, ecx, eax;
    uint32_t int_no, err_code;
    uint32_t eip, cs, eflags, useresp, ss;
} registers_t;
```

1. **CPU Exception Handler (`isr_handler`)**
   - Catches faults (Divide by Zero, Invalid Opcode, General Protection Fault, Page Fault).
   - Dumps registers (`EIP`, `EFLAGS`, `CS`, `EAX`..`EDI`, `CR2` for page faults) to both the screen and the serial debug port.
   - Halts the CPU if a kernel-mode crash is unrecoverable.

2. **Hardware IRQ Dispatcher (`irq_handler`)**
   - Dispatches to registered custom callback handlers (e.g. Timer, Keyboard, Mouse, Network).
   - Issues End-of-Interrupt (EOI) to Slave PIC (`outb(0xA0, 0x20)`) if `int_no >= 40`.
   - Issues EOI to Master PIC (`outb(0x20, 0x20)`).
