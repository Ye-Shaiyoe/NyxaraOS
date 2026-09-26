#include "gdt.h"

extern void load_gdt_asm(uint32_t gdt_ptr);
extern void tss_flush_asm(void);

static gdt_entry_t gdt_entries[6];
static gdt_ptr_t   gdt_ptr;
static tss_entry_t tss_entry;

void gdt_set_gate(int32_t num, uint32_t base, uint32_t limit, uint8_t access, uint8_t gran) {
    gdt_entries[num].base_low    = (base & 0xFFFF);
    gdt_entries[num].base_middle = (base >> 16) & 0xFF;
    gdt_entries[num].base_high   = (base >> 24) & 0xFF;

    gdt_entries[num].limit_low   = (limit & 0xFFFF);
    gdt_entries[num].granularity = (limit >> 16) & 0x0F;

    gdt_entries[num].granularity |= gran & 0xF0;
    gdt_entries[num].access      = access;
}

void tss_set_stack(uint32_t ss0, uint32_t esp0) {
    tss_entry.ss0  = ss0;
    tss_entry.esp0 = esp0;
}

static void write_tss(int32_t num, uint16_t ss0, uint32_t esp0) {
    uint32_t base = (uint32_t)&tss_entry;
    uint32_t limit = sizeof(tss_entry_t) - 1;

    gdt_set_gate(num, base, limit, 0x89, 0x00);

    memset(&tss_entry, 0, sizeof(tss_entry_t));
    tss_entry.ss0  = ss0;
    tss_entry.esp0 = esp0;
    tss_entry.cs   = 0x08 | 3;
    tss_entry.ss   = 0x10 | 3;
    tss_entry.ds   = 0x10 | 3;
    tss_entry.es   = 0x10 | 3;
    tss_entry.fs   = 0x10 | 3;
    tss_entry.gs   = 0x10 | 3;
}

void gdt_init(void) {
    gdt_ptr.limit = (sizeof(gdt_entry_t) * 6) - 1;
    gdt_ptr.base  = (uint32_t)&gdt_entries;

    // 0: Null descriptor
    gdt_set_gate(0, 0, 0, 0, 0);

    // 1: Kernel Code Segment (0x08)
    gdt_set_gate(1, 0, 0xFFFFFFFF, 0x9A, 0xCF);

    // 2: Kernel Data Segment (0x10)
    gdt_set_gate(2, 0, 0xFFFFFFFF, 0x92, 0xCF);

    // 3: User Code Segment (0x18)
    gdt_set_gate(3, 0, 0xFFFFFFFF, 0xFA, 0xCF);

    // 4: User Data Segment (0x20)
    gdt_set_gate(4, 0, 0xFFFFFFFF, 0xF2, 0xCF);

    // 5: Task State Segment (0x28)
    write_tss(5, 0x10, 0x00140000);

    load_gdt_asm((uint32_t)&gdt_ptr);
    tss_flush_asm();
}

