#include "acpi.h"
#include "../hal/io.h"
#include "../hal/serial.h"

static acpi_info_t acpi_state;

static bool validate_checksum(const void* ptr, size_t length) {
    const uint8_t* b = (const uint8_t*)ptr;
    uint8_t sum = 0;
    for (size_t i = 0; i < length; i++) {
        sum += b[i];
    }
    return sum == 0;
}

static acpi_rsdp_t* find_rsdp_in_range(uint32_t start, uint32_t end) {
    for (uint32_t addr = start; addr < end; addr += 16) {
        if (memcmp((const void*)addr, "RSD PTR ", 8) == 0) {
            acpi_rsdp_t* rsdp = (acpi_rsdp_t*)addr;
            if (validate_checksum(rsdp, 20)) {
                return rsdp;
            }
        }
    }
    return NULL;
}

static acpi_rsdp_t* find_rsdp(void) {
    uint16_t ebda_seg = 0;
    __asm__ volatile("movw 0x040E, %0" : "=r"(ebda_seg));
    uint32_t ebda_addr = ((uint32_t)ebda_seg) << 4;
    acpi_rsdp_t* rsdp = NULL;

    if (ebda_addr >= 0x80000 && ebda_addr < 0xA0000) {
        rsdp = find_rsdp_in_range(ebda_addr, ebda_addr + 1024);
        if (rsdp) return rsdp;
    }

    return find_rsdp_in_range(0x000E0000, 0x00100000);
}

static void parse_madt(acpi_madt_t* madt) {
    acpi_state.madt = madt;
    acpi_state.lapic_addr = madt->lapic_addr;
    acpi_state.has_legacy_pic = (madt->flags & 1) != 0;

    uint8_t* ptr = (uint8_t*)madt + sizeof(acpi_madt_t);
    uint8_t* end = (uint8_t*)madt + madt->header.length;

    while (ptr < end) {
        acpi_madt_entry_header_t* entry = (acpi_madt_entry_header_t*)ptr;
        if (entry->length == 0) {
            break;
        }

        switch (entry->type) {
            case 0: { // Processor Local APIC
                acpi_madt_lapic_t* lapic = (acpi_madt_lapic_t*)entry;
                if ((lapic->flags & 1) || (lapic->flags & 2)) {
                    if (acpi_state.cpu_count < ACPI_MAX_CPUS) {
                        acpi_state.cpu_apic_ids[acpi_state.cpu_count++] = lapic->apic_id;
                    }
                }
                break;
            }
            case 1: { // I/O APIC
                acpi_madt_ioapic_t* ioapic = (acpi_madt_ioapic_t*)entry;
                if (acpi_state.ioapic_addr == 0) {
                    acpi_state.ioapic_addr = ioapic->ioapic_addr;
                    acpi_state.ioapic_gsi_base = ioapic->gsi_base;
                }
                break;
            }
            default:
                break;
        }

        ptr += entry->length;
    }
}

int acpi_init(void) {
    memset(&acpi_state, 0, sizeof(acpi_state));

    acpi_rsdp_t* rsdp = find_rsdp();
    if (!rsdp) {
        serial_puts("[ACPI] RSDP signature not found in EBDA or BIOS memory.\n");
        return -1;
    }

    serial_puts("[ACPI] RSDP located at physical memory.\n");

    acpi_rsdt_t* rsdt = (acpi_rsdt_t*)rsdp->rsdt_addr;
    if (!rsdt || memcmp(rsdt->header.signature, "RSDT", 4) != 0) {
        serial_puts("[ACPI] Invalid RSDT header signature.\n");
        return -2;
    }

    if (!validate_checksum(rsdt, rsdt->header.length)) {
        serial_puts("[ACPI] RSDT checksum verification failed.\n");
        return -3;
    }

    size_t table_count = (rsdt->header.length - sizeof(acpi_sdt_header_t)) / sizeof(uint32_t);

    for (size_t i = 0; i < table_count; i++) {
        acpi_sdt_header_t* header = (acpi_sdt_header_t*)rsdt->tables[i];
        if (!header) continue;

        if (!validate_checksum(header, header->length)) {
            continue;
        }

        if (memcmp(header->signature, "FACP", 4) == 0) {
            acpi_state.fadt = (acpi_fadt_t*)header;
            serial_puts("[ACPI] FADT (Fixed ACPI Description Table) detected.\n");
        } else if (memcmp(header->signature, "APIC", 4) == 0) {
            parse_madt((acpi_madt_t*)header);
            serial_puts("[ACPI] MADT parsed: Local APIC & CPU topology recorded.\n");
        }
    }

    acpi_state.initialized = true;
    return 0;
}

const acpi_info_t* acpi_get_info(void) {
    return &acpi_state;
}

uint32_t acpi_get_lapic_addr(void) {
    return acpi_state.lapic_addr;
}

uint32_t acpi_get_ioapic_addr(void) {
    return acpi_state.ioapic_addr;
}

uint32_t acpi_get_cpu_count(void) {
    return acpi_state.cpu_count;
}

bool acpi_has_legacy_pic(void) {
    return acpi_state.has_legacy_pic;
}

void acpi_reboot(void) {
    if (acpi_state.fadt && (acpi_state.fadt->flags & (1 << 10))) {
        if (acpi_state.fadt->reset_reg.address_space == 1) {
            outb((uint16_t)acpi_state.fadt->reset_reg.address, acpi_state.fadt->reset_value);
        }
    }

    // 8042 keyboard controller reset fallback
    uint8_t good = 0x02;
    while (good & 0x02) {
        good = inb(0x64);
    }
    outb(0x64, 0xFE);

    // Triple fault fallback
    struct {
        uint16_t limit;
        uint32_t base;
    } __attribute__((packed)) null_idt = {0, 0};

    __asm__ volatile("lidt %0; int3" : : "m"(null_idt));
}

void acpi_poweroff(void) {
    if (acpi_state.fadt && acpi_state.fadt->pm1a_cnt_blk) {
        // Typical ACPI sleep enable bit: SLP_TYPa (bits 10-12) | SLP_EN (bit 13)
        outw((uint16_t)acpi_state.fadt->pm1a_cnt_blk, 0x2000 | (5 << 10));
        if (acpi_state.fadt->pm1b_cnt_blk) {
            outw((uint16_t)acpi_state.fadt->pm1b_cnt_blk, 0x2000 | (5 << 10));
        }
    }

    // QEMU legacy poweroff exit port
    outw(0x604, 0x2000);
    outw(0xB004, 0x2000);

    serial_puts("[ACPI] Poweroff triggered. System halting.\n");
    while (1) {
        hlt();
    }
}
