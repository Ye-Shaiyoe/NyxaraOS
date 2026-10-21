#include "../hal/hal.h"
#include "acpi.h"

// Rust entry point declaration
extern void nyxara_rust_main(void);

void hal_init(boot_info_t* boot_info) {
    // 1. Initialize Global Descriptor Table
    gdt_init();

    // 2. Initialize Interrupt Descriptor Table & Remap PIC 8259
    idt_init();

    // 3. Initialize Framebuffer or fallback to VGA text mode
    fb_init(boot_info);
    if (!fb_is_active()) {
        vga_init();
    }

    // 4. Initialize Serial Port COM1 (38400 baud) for debug logging
    serial_init();
    serial_puts("\n[Nyxara Kernel] Serial COM1 logging initialized.\n");

    // 5. Initialize PIT Timer (100Hz)
    timer_init(100);
    serial_puts("[Nyxara Kernel] PIT Timer initialized (100Hz).\n");

    // 6. Initialize PS/2 Keyboard Driver
    keyboard_init();
    serial_puts("[Nyxara Kernel] PS/2 Keyboard driver initialized.\n");

    // 7. Initialize PS/2 Mouse Driver
    mouse_init();
    serial_puts("[Nyxara Kernel] PS/2 Mouse driver initialized.\n");

    // 8. Initialize PCI Bus & RTL8139 Network Card
    pci_init();
    rtl8139_init();

    // 9. Initialize RTC/CMOS Real-Time Clock
    rtc_init();
    serial_puts("[Nyxara Kernel] RTC/CMOS driver initialized.\n");

    // 10. Initialize ACPI (RSDP, RSDT, FADT, MADT)
    acpi_init();

    // 11. Enable Interrupts (STI)
    sti();
    serial_puts("[Nyxara Kernel] Hardware interrupts enabled (STI).\n");
}

void kmain(boot_info_t* boot_info) {
    // Inisialisasi Hardware Abstraction Layer
    hal_init(boot_info);

    serial_puts("[Nyxara Kernel] HAL initialization complete. Handing over to Rust Kernel Core...\n");

    // Masuk ke Rust Kernel Core & Shell dengan stack yang selaras 16-byte (System V ABI)
    __asm__ volatile (
        "movl $0x00140000, %%esp\n\t"
        "andl $0xFFFFFFF0, %%esp\n\t"
        "subl $12, %%esp\n\t"
        "call nyxara_rust_main\n\t"
        : : : "memory"
    );

    // Jika shell Rust selesai/keluar, masuk ke mode idle CPU halt
    serial_puts("[Nyxara Kernel] Rust main returned. System entering idle state.\n");
    while (1) {
        hlt();
    }
}
