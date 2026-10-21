# ACPI Subsystem & Hardware Topology Discovery

The Advanced Configuration and Power Interface (ACPI) subsystem resides in `kernel/acpi.c` and `kernel/acpi.h`. It is responsible for discovering bare-metal platform topology, detecting Multi-core CPU layouts via the Multiple APIC Description Table (MADT), and providing hardware-level power management (reboot and shutdown), serving as a foundation for target laptop hardware like the ThinkPad T480.

## Discovery Workflow

```
+-------------------------------------------------------------+
| EBDA (0x80000..0x9FFFF) or BIOS ROM Area (0x000E0000..0x000FFFFF) |
|                     Scan for "RSD PTR "                     |
+-------------------------------------------------------------+
                               |
                               v
+-------------------------------------------------------------+
|                 Validate 20-byte Checksum                   |
|                   Extract RSDT Pointer                      |
+-------------------------------------------------------------+
                               |
                               v
+-------------------------------------------------------------+
|                 Iterate SDT Tables in RSDT                  |
|    +-------------------------+   +-------------------------+|
|    |    "FACP" (FADT)        |   |    "APIC" (MADT)        ||
|    | - Reset Register        |   | - Local APIC Address    ||
|    | - PM1a/PM1b Control Blk |   | - CPU Cores & APIC IDs  ||
|    | - ACPI Reboot / S5 Off  |   | - I/O APIC Address/GSI  ||
|    +-------------------------+   +-------------------------+|
+-------------------------------------------------------------+
```

## Structure Contracts

### 1. Root System Description Pointer (`acpi_rsdp_t`)
The RSDP structure is located by scanning EBDA (address fetched from BDA `0x040E`) and the memory window `0x000E0000` to `0x000FFFFF` on 16-byte boundaries.

```c
typedef struct {
    char     signature[8]; // "RSD PTR "
    uint8_t  checksum;
    char     oem_id[6];
    uint8_t  revision;
    uint32_t rsdt_addr;
    uint32_t length;
    uint64_t xsdt_addr;
    uint8_t  extended_checksum;
    uint8_t  reserved[3];
} __attribute__((packed)) acpi_rsdp_t;
```

### 2. Multiple APIC Description Table (`acpi_madt_t`)
MADT identifies the modern interrupt architecture that replaces the legacy PIC 8259:
* Local APIC physical address (default `0xFEE00000`).
* Dual-8259 legacy compatibility flags (`flags & 1`).
* Variable-length entries:
  * **Type 0 (`acpi_madt_lapic_t`)**: Processor Local APIC. Identifies enabled CPU cores and their hardware APIC IDs.
  * **Type 1 (`acpi_madt_ioapic_t`)**: I/O APIC controller address (default `0xFEC00000`) and GSI base.
  * **Type 2 (`acpi_madt_iso_t`)**: Interrupt Source Overrides (e.g. mapping ISA IRQ0 timer to APIC pin).

### 3. Fixed ACPI Description Table (`acpi_fadt_t`)
FADT provides system reset registers and power management blocks:
* `reset_reg` (Generic Address Structure) & `reset_value`: Triggers instant hardware reboot without going through BIOS/8042 controller.
* `pm1a_cnt_blk` & `pm1b_cnt_blk`: Controls ACPI system sleep transitions (S5 soft-off).

## Kernel API

| Function | Description |
|---|---|
| `int acpi_init(void)` | Locates RSDP, validates RSDT checksum, and parses FADT/MADT tables |
| `const acpi_info_t* acpi_get_info(void)` | Returns the system ACPI status structure |
| `uint32_t acpi_get_lapic_addr(void)` | Returns physical base address of Local APIC |
| `uint32_t acpi_get_ioapic_addr(void)` | Returns physical base address of I/O APIC |
| `uint32_t acpi_get_cpu_count(void)` | Returns number of enabled logical CPU cores |
| `bool acpi_has_legacy_pic(void)` | Indicates whether legacy PIC 8259 exists on board |
| `void acpi_reboot(void)` | Executes system reboot via ACPI reset register (fallback: 8042/triple fault) |
| `void acpi_poweroff(void)` | Executes ACPI S5 sleep transition / power off |
