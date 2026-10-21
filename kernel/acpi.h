#ifndef NYXARA_KERNEL_ACPI_H
#define NYXARA_KERNEL_ACPI_H

#include "../hal/types.h"

#define ACPI_MAX_CPUS 32

typedef struct {
    char     signature[8];
    uint8_t  checksum;
    char     oem_id[6];
    uint8_t  revision;
    uint32_t rsdt_addr;
    uint32_t length;
    uint64_t xsdt_addr;
    uint8_t  extended_checksum;
    uint8_t  reserved[3];
} __attribute__((packed)) acpi_rsdp_t;

typedef struct {
    char     signature[4];
    uint32_t length;
    uint8_t  revision;
    uint8_t  checksum;
    char     oem_id[6];
    char     oem_table_id[8];
    uint32_t oem_revision;
    uint32_t creator_id;
    uint32_t creator_revision;
} __attribute__((packed)) acpi_sdt_header_t;

typedef struct {
    acpi_sdt_header_t header;
    uint32_t tables[];
} __attribute__((packed)) acpi_rsdt_t;

typedef struct {
    uint8_t  address_space;
    uint8_t  bit_width;
    uint8_t  bit_offset;
    uint8_t  access_size;
    uint64_t address;
} __attribute__((packed)) acpi_gas_t;

typedef struct {
    acpi_sdt_header_t header;
    uint32_t firmware_ctrl;
    uint32_t dsdt;
    uint8_t  reserved;
    uint8_t  preferred_pm_profile;
    uint16_t sci_int;
    uint32_t smi_cmd;
    uint8_t  acpi_enable;
    uint8_t  acpi_disable;
    uint8_t  s4bios_req;
    uint8_t  pstate_cnt;
    uint32_t pm1a_evt_blk;
    uint32_t pm1b_evt_blk;
    uint32_t pm1a_cnt_blk;
    uint32_t pm1b_cnt_blk;
    uint32_t pm2_cnt_blk;
    uint32_t pm_tmr_blk;
    uint32_t gpe0_blk;
    uint32_t gpe1_blk;
    uint8_t  pm1_evt_len;
    uint8_t  pm1_cnt_len;
    uint8_t  pm2_cnt_len;
    uint8_t  pm_tmr_len;
    uint8_t  gpe0_len;
    uint8_t  gpe1_len;
    uint8_t  gpe1_base;
    uint8_t  cst_cnt;
    uint16_t p_lvl2_lat;
    uint16_t p_lvl3_lat;
    uint16_t flush_size;
    uint16_t flush_stride;
    uint8_t  duty_offset;
    uint8_t  duty_width;
    uint8_t  day_alrm;
    uint8_t  mon_alrm;
    uint8_t  century;
    uint16_t iapc_boot_arch;
    uint8_t  reserved1;
    uint32_t flags;
    acpi_gas_t reset_reg;
    uint8_t  reset_value;
    uint8_t  reserved2[3];
} __attribute__((packed)) acpi_fadt_t;

typedef struct {
    acpi_sdt_header_t header;
    uint32_t lapic_addr;
    uint32_t flags;
} __attribute__((packed)) acpi_madt_t;

typedef struct {
    uint8_t type;
    uint8_t length;
} __attribute__((packed)) acpi_madt_entry_header_t;

typedef struct {
    acpi_madt_entry_header_t header;
    uint8_t  acpi_processor_id;
    uint8_t  apic_id;
    uint32_t flags;
} __attribute__((packed)) acpi_madt_lapic_t;

typedef struct {
    acpi_madt_entry_header_t header;
    uint8_t  ioapic_id;
    uint8_t  reserved;
    uint32_t ioapic_addr;
    uint32_t gsi_base;
} __attribute__((packed)) acpi_madt_ioapic_t;

typedef struct {
    acpi_madt_entry_header_t header;
    uint8_t  bus;
    uint8_t  source_irq;
    uint32_t gsi;
    uint16_t flags;
} __attribute__((packed)) acpi_madt_iso_t;

typedef struct {
    bool     initialized;
    uint32_t lapic_addr;
    uint32_t ioapic_addr;
    uint32_t ioapic_gsi_base;
    uint32_t cpu_count;
    uint8_t  cpu_apic_ids[ACPI_MAX_CPUS];
    bool     has_legacy_pic;
    acpi_fadt_t* fadt;
    acpi_madt_t* madt;
} acpi_info_t;

int acpi_init(void);
const acpi_info_t* acpi_get_info(void);
uint32_t acpi_get_lapic_addr(void);
uint32_t acpi_get_ioapic_addr(void);
uint32_t acpi_get_cpu_count(void);
bool acpi_has_legacy_pic(void);
void acpi_reboot(void);
void acpi_poweroff(void);

#endif // NYXARA_KERNEL_ACPI_H
