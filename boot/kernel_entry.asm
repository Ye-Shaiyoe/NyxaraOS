; ==============================================================================
; Nyxara OS - Kernel Entry & ISR Handlers
; 32-bit Protected Mode Assembly Glue
; ==============================================================================

[BITS 32]

global _start
global load_gdt_asm
global load_idt_asm
global tss_flush_asm
global isr128
global isr0, isr1, isr2, isr3, isr4, isr5, isr6, isr7
global isr8, isr9, isr10, isr11, isr12, isr13, isr14, isr15
global isr16, isr17, isr18, isr19, isr20, isr21, isr22, isr23
global isr24, isr25, isr26, isr27, isr28, isr29, isr30, isr31

global irq0, irq1, irq2, irq3, irq4, irq5, irq6, irq7
global irq8, irq9, irq10, irq11, irq12, irq13, irq14, irq15

extern kmain
extern isr_handler
extern irq_handler
extern irq_switch_stack
extern bss_start
extern bss_end

; Kernel stack placed at 0x00130000..0x00140000 (64 KB in extended DRAM above 1MB)
KERNEL_STACK_TOP equ 0x00140000

SECTION .text

; ------------------------------------------------------------------------------
; Kernel Entry Point
; ------------------------------------------------------------------------------
_start:
    ; Pastikan segment data 0x10 sudah diset
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Setup initial stack pointer in extended DRAM
    mov esp, KERNEL_STACK_TOP

    ; Zero out the entire .bss section
    mov edi, bss_start
    mov ecx, bss_end
    sub ecx, edi
    xor eax, eax
    cld
    rep stosb

    ; Re-anchor stack top after BSS zeroing
    mov esp, KERNEL_STACK_TOP

    ; Aktifkan FPU & SSE pada CR0 dan CR4
    mov eax, cr0
    and eax, ~(1 << 2)          ; Clear EM (Emulation)
    or eax, (1 << 1)           ; Set MP (Monitor Coprocessor)
    mov cr0, eax

    mov eax, cr4
    or eax, (3 << 9)           ; Set OSFXSR (bit 9) dan OSXMMEXCPT (bit 10)
    mov cr4, eax

    ; Inisialisasi unit FPU
    fninit

    ; Panggil fungsi utama C kernel dengan pointer BootInfo (0x9000)
    push 0x9000
    call kmain
    add esp, 4

    ; Jika kmain kembali, lakukan halt CPU
.hang:
    cli
    hlt
    jmp .hang

; ------------------------------------------------------------------------------
; GDT Loader Helper
; ------------------------------------------------------------------------------
load_gdt_asm:
    mov eax, [esp + 4]          ; Pointer ke GDT descriptor
    lgdt [eax]

    ; Far jump untuk reload CS segment (0x08)
    jmp 0x08:.reload_cs
.reload_cs:
    mov ax, 0x10                ; 0x10 = data segment selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    ret

; ------------------------------------------------------------------------------
; IDT Loader Helper
; ------------------------------------------------------------------------------
load_idt_asm:
    mov eax, [esp + 4]          ; Pointer ke IDT descriptor
    lidt [eax]
    ret

tss_flush_asm:
    mov ax, 0x28                ; Index 5 in GDT
    ltr ax
    ret

; Minimal Ring 3 proof-of-life. The function is copied to a user page before use.
global user_demo_start
global user_demo_end
section .user_text
user_demo_start:
    mov eax, 20                 ; SYS_GETPID
    int 0x80
.loop:
    pause
    jmp .loop
user_demo_end:

; ISR 128 (0x80 System Call)
isr128:
    push 0                      ; Dummy error code
    push 128                    ; Interrupt vector 128
    jmp isr_common_stub


; ISR Exception Macros

%macro ISR_NOERRCODE 1
    global isr%1
    isr%1:
        push 0                  ; Dummy error code
        push %1                 ; Interrupt number
        jmp isr_common_stub
%endmacro

%macro ISR_ERRCODE 1
    global isr%1
    isr%1:
        push %1                 ; Interrupt number (error code sudah dipush CPU)
        jmp isr_common_stub
%endmacro

; ISR Definitions (0 - 31)
ISR_NOERRCODE 0
ISR_NOERRCODE 1
ISR_NOERRCODE 2
ISR_NOERRCODE 3
ISR_NOERRCODE 4
ISR_NOERRCODE 5
ISR_NOERRCODE 6
ISR_NOERRCODE 7
ISR_ERRCODE   8
ISR_NOERRCODE 9
ISR_ERRCODE   10
ISR_ERRCODE   11
ISR_ERRCODE   12
ISR_ERRCODE   13
ISR_ERRCODE   14
ISR_NOERRCODE 15
ISR_NOERRCODE 16
ISR_ERRCODE   17
ISR_NOERRCODE 18
ISR_NOERRCODE 19
ISR_NOERRCODE 20
ISR_NOERRCODE 21
ISR_NOERRCODE 22
ISR_NOERRCODE 23
ISR_NOERRCODE 24
ISR_NOERRCODE 25
ISR_NOERRCODE 26
ISR_NOERRCODE 27
ISR_NOERRCODE 28
ISR_NOERRCODE 29
ISR_NOERRCODE 30
ISR_NOERRCODE 31

; Common ISR Stub
isr_common_stub:
    pusha                       ; Push EDI, ESI, EBP, ESP, EBX, EDX, ECX, EAX

    mov ax, ds                  ; Simpan data segment descriptor
    push eax

    mov ax, 0x10                ; Muat kernel data segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    push esp                    ; Pass pointer ke struct registers_t ke fungsi C
    call isr_handler
    add esp, 4                  ; Cleanup pointer argument

    pop eax                     ; Restore original data segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    popa                        ; Restore register umum
    add esp, 8                  ; Pop interrupt number dan error code
    iret

; ------------------------------------------------------------------------------
; IRQ Macros (Hardware Interrupts 0..15 -> Remapped to 32..47)
; ------------------------------------------------------------------------------
%macro IRQ 2
    global irq%1
    irq%1:
        push 0                  ; Dummy error code
        push %2                 ; Remapped interrupt number (32..47)
        jmp irq_common_stub
%endmacro

IRQ 0, 32
IRQ 1, 33
IRQ 2, 34
IRQ 3, 35
IRQ 4, 36
IRQ 5, 37
IRQ 6, 38
IRQ 7, 39
IRQ 8, 40
IRQ 9, 41
IRQ 10, 42
IRQ 11, 43
IRQ 12, 44
IRQ 13, 45
IRQ 14, 46
IRQ 15, 47

; Common IRQ Stub
irq_common_stub:
    pusha

    mov ax, ds
    push eax

    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    push esp
    call irq_handler
    add esp, 4
    mov edx, [irq_switch_stack]
    test edx, edx
    jz .use_saved_frame

    ; Copy the iret frame to the task's real stack before restoring registers.
    ; Load all values first because the destination overlaps the source frame.
    mov ecx, [eax + 44]         ; EIP
    mov edi, [eax + 48]         ; CS
    mov esi, [eax + 52]         ; EFLAGS
    mov [edx], ecx
    mov [edx + 4], edi
    mov [edx + 8], esi
    test edi, 3
    jz .use_saved_frame
    mov ecx, [eax + 56]          ; User ESP
    mov esi, [eax + 60]          ; User SS
    mov [edx + 12], ecx
    mov [edx + 16], esi

.use_saved_frame:
    mov esp, eax

    pop eax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    popa
    add esp, 8
    cmp dword [irq_switch_stack], 0
    je .return_from_saved_frame
    mov esp, [irq_switch_stack]
    mov dword [irq_switch_stack], 0
.return_from_saved_frame:
    iret
