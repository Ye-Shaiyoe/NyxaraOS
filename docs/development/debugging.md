# Debugging Guide

Debugging bare-metal operating system software requires specialized strategies since standard userland tooling and print debuggers are unavailable during low-level execution. This guide details Nyxara's multi-tiered debugging workflows.

## 1. Remote GDB Debugging via QEMU

Nyxara OS includes a pre-configured Makefile target that launches QEMU with a GDB remote debugging server:

```bash
make debug
```

QEMU initializes in a halted state (`-S`), listening on TCP port `1234` (`-s`).

### Connecting from GDB
In a separate terminal window, launch GDB and connect to the QEMU instance, loading the symbol table from the compiled ELF binary:

```bash
gdb -ex "target remote localhost:1234" -ex "symbol-file build/kernel.elf"
```

### Useful GDB Commands for Kernel Inspection

- **Breakpoints**:
  ```gdb
  b _start                 # Break at assembly entry point
  b kmain                  # Break at C entry point
  b nyxara_rust_main       # Break at Rust core entry point
  b page_fault_handler     # Break at Page Fault exception handler
  b handle_command         # Break at shell command dispatcher
  ```
- **Execution Control**:
  ```gdb
  c                        # Continue execution
  si                       # Step single assembly instruction
  n                        # Next source line
  ```
- **CPU Registers**:
  ```gdb
  info registers           # Dump general purpose and segment registers
  p/x $cr0                 # Inspect CR0 (Paging, Protection bits)
  p/x $cr2                 # Inspect CR2 (Page Fault linear address)
  p/x $cr3                 # Inspect CR3 (Page Directory base pointer)
  ```
- **Memory Inspection**:
  ```gdb
  x/16xw 0x10000           # Examine 16 words at kernel base
  x/8xw 0x00100000         # Examine Page Directory entries
  x/32xb 0x9000            # Examine BootInfo struct
  ```

---

## 2. Serial Port Logging (COM1)

Every subsystem logs operational milestones and error conditions through UART COM1 (`0x3F8` @ 38400 baud):

### Monitoring in Real Time
```bash
make run-serial
```
Serial log lines are piped directly to your terminal's standard output.

### Log File Inspection
QEMU sessions can also redirect serial output to disk:
```bash
tail -f serial.log
```

### Logging Macros
- In C HAL: `serial_puts(const char* str)`, `serial_puthex(uint32_t val)`.
- In Rust Core: `log!(...)` and `logln!(...)` macros write formatted strings directly to COM1.

---

## 3. Diagnosing CPU Exceptions

When the CPU encounters a fault (e.g. Invalid Opcode `#UD`, General Protection Fault `#GP`, or unmapped Page Fault `#PF`), the assembly trampoline invokes `isr_handler()` in `hal/isr.c`:

```
====================================================
[CPU EXCEPTION] Vector: 14 (Page Fault) Error: 0x0002
CR2 (Faulting Linear Address): 0xC0002000
EIP: 0x000185A4  CS:  0x0008  EFLAGS: 0x00010046
EAX: 0x00000000  EBX: 0x00020000  ECX: 0x00000001
EDX: 0x00140000  ESI: 0x0001A020  EDI: 0x0001A040
EBP: 0x0013FFE0  ESP: 0x0013FFC8  DS:  0x0010
====================================================
```

### Decoding Page Fault Error Codes (ISR 14)
- **Bit 0 (`P = 0`)**: The fault was caused by a **not-present page** (e.g. demand paging trigger or null pointer dereference).
- **Bit 0 (`P = 1`)**: Page protection violation (e.g. writing to read-only page).
- **Bit 1 (`W/R = 1`)**: Caused by a **write** instruction.
- **Bit 2 (`U/S = 1`)**: Triggered from **User Mode (Ring 3)**.

---

## 4. Diagnosing Rust Kernel Panics

When an explicit invariant fails in Rust (e.g. out-of-bounds slice indexing or `unwrap()` on `None`):
1. The screen transitions to high-contrast White on Red.
2. The panic handler prints the source file path and line number.
3. The identical message is logged to COM1 serial:
   ```
   [Nyxara Kernel Panic] panicked at 'assertion failed: index < len', rust/src/commands.rs:342
   ```
4. The CPU executes `cli; hlt` to freeze execution and prevent data corruption.
