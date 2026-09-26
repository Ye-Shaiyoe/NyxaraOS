/* ==============================================================================
 * Nyxara OS - Linker Script
 * Combines Assembly entry, C HAL drivers, and Rust kernel staticlib
 * ============================================================================== */

OUTPUT_FORMAT(elf32-i386)
OUTPUT_ARCH(i386)
ENTRY(_start)

SECTIONS
{
    . = 0x10000;
    kernel_start = .;

    .text : ALIGN(4K)
    {
        *(.text._start)
        *(.text)
        *(.text.*)
    }

    .user_text : ALIGN(4K)
    {
        *(.user_text)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata)
        *(.rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data)
        *(.data.*)
    }

    .bss : ALIGN(4K)
    {
        bss_start = .;
        *(COMMON)
        *(.bss)
        *(.bss.*)
        bss_end = .;
    }

    . = ALIGN(4K);
    kernel_end = .;

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.note.*)
    }
}

