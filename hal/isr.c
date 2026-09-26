#include "isr.h"
#include "vga.h"
#include "io.h"
#include "serial.h"

static isr_t interrupt_handlers[256] = {0};
static registers_t* switch_frame = 0;
uint32_t irq_switch_stack = 0;

static const char* const exception_messages[32] = {
    "Divide by Zero",
    "Debug",
    "Non-Maskable Interrupt",
    "Breakpoint",
    "Overflow",
    "Bound Range Exceeded",
    "Invalid Opcode",
    "Device Not Available",
    "Double Fault",
    "Coprocessor Segment Overrun",
    "Invalid TSS",
    "Segment Not Present",
    "Stack-Segment Fault",
    "General Protection Fault",
    "Page Fault",
    "Reserved",
    "x87 Floating-Point Exception",
    "Alignment Check",
    "Machine Check",
    "SIMD Floating-Point Exception",
    "Virtualization Exception",
    "Control Protection Exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Hypervisor Injection Exception",
    "VMM Communication Exception",
    "Security Exception",
    "Reserved"
};

void isr_register_handler(uint8_t n, isr_t handler) {
    interrupt_handlers[n] = handler;
}

void isr_handler(registers_t* regs) {
    if (interrupt_handlers[regs->int_no] != 0) {
        isr_t handler = interrupt_handlers[regs->int_no];
        handler(regs);
        return;
    }

    vga_set_color(VGA_COLOR_WHITE, VGA_COLOR_RED);
    vga_puts("\n[CPU EXCEPTION PANIC]\n");
    serial_puts("\n[CPU EXCEPTION PANIC]\nException: ");
    vga_puts("Exception: ");
    if (regs->int_no < 32) {
        vga_puts(exception_messages[regs->int_no]);
        serial_puts(exception_messages[regs->int_no]);
    } else {
        vga_puts("Unknown Interrupt");
        serial_puts("Unknown Interrupt");
    }
    vga_puts(" (Vector: ");
    serial_puts(" (Vector: ");
    vga_putdec(regs->int_no);
    serial_putdec(regs->int_no);
    vga_puts(", ErrCode: ");
    serial_puts(", ErrCode: ");
    vga_puthex(regs->err_code);
    serial_puthex(regs->err_code);
    vga_puts(")\n");
    serial_puts(")\n");

    vga_puts("EIP: "); vga_puthex(regs->eip);
    serial_puts("EIP: "); serial_puthex(regs->eip);
    vga_puts(" CS: ");  vga_puthex(regs->cs);
    serial_puts(" CS: ");  serial_puthex(regs->cs);
    vga_puts(" EFLAGS: "); vga_puthex(regs->eflags);
    serial_puts(" EFLAGS: "); serial_puthex(regs->eflags);
    vga_puts("\n");
    serial_puts("\n");

    vga_puts("EAX: "); vga_puthex(regs->eax);
    serial_puts("EAX: "); serial_puthex(regs->eax);
    vga_puts(" EBX: "); vga_puthex(regs->ebx);
    serial_puts(" EBX: "); serial_puthex(regs->ebx);
    vga_puts(" ECX: "); vga_puthex(regs->ecx);
    serial_puts(" ECX: "); serial_puthex(regs->ecx);
    vga_puts(" EDX: "); vga_puthex(regs->edx);
    serial_puts(" EDX: "); serial_puthex(regs->edx);
    vga_puts("\n");
    serial_puts("\n");

    vga_puts("ESP: "); vga_puthex(regs->esp);
    serial_puts("ESP: "); serial_puthex(regs->esp);
    vga_puts(" EBP: "); vga_puthex(regs->ebp);
    serial_puts(" EBP: "); serial_puthex(regs->ebp);
    vga_puts(" ESI: "); vga_puthex(regs->esi);
    serial_puts(" ESI: "); serial_puthex(regs->esi);
    vga_puts(" EDI: "); vga_puthex(regs->edi);
    serial_puts(" EDI: "); serial_puthex(regs->edi);
    vga_puts("\nSystem halted. Please reset.\n");
    serial_puts("\nSystem halted. Please reset.\n");

    cli();
    while (1) {
        hlt();
    }
}

void irq_set_switch_frame(registers_t* regs, uint32_t stack_top) {
    switch_frame = regs;
    if (stack_top == 0) {
        irq_switch_stack = 0;
        return;
    }

    irq_switch_stack = (regs->cs & 3) == 3 ? stack_top - 20 : stack_top - 12;
}

registers_t* irq_handler(registers_t* regs) {
    switch_frame = 0;
    irq_switch_stack = 0;

    // Send EOI (End of Interrupt) to PICs
    if (regs->int_no >= 40) {
        // Send reset signal to slave
        outb(0xA0, 0x20);
    }
    // Send reset signal to master
    outb(0x20, 0x20);

    if (interrupt_handlers[regs->int_no] != 0) {
        isr_t handler = interrupt_handlers[regs->int_no];
        handler(regs);
    }

    return switch_frame != 0 ? switch_frame : regs;
}
