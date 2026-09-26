# Kernel Core Overview

The Nyxara Kernel Core (`rust/src/`) is written in freestanding Rust (`#![no_std]`, `#![no_main]`), leveraging the language's safety guarantees to manage hardware resources, page tables, system calls, and virtual filesystems without an underlying C runtime or standard library. The process scheduler and Ring 3 proof-of-life task are described in [Processes, Scheduling, and Ring 3](processes.md); they are prototypes rather than complete process isolation.

## Entry Point Transition (`nyxara_rust_main`)

Execution transitions from C `kmain` via an assembly trampoline that aligns the stack pointer to a 16-byte boundary to satisfy the System V i386 ABI:

```c
__asm__ volatile (
    "movl $0x00140000, %%esp\n\t"
    "andl $0xFFFFFFF0, %%esp\n\t"
    "subl $12, %%esp\n\t"
    "call nyxara_rust_main\n\t"
    : : : "memory"
);
```

In `rust/src/lib.rs`, `nyxara_rust_main()` assumes control:

```rust
#[no_mangle]
pub extern "C" fn nyxara_rust_main() -> ! {
    vga::clear_screen();

    // 1. Determine kernel boundaries from linker script
    let k_start = unsafe { &kernel_start as *const u8 as usize };
    let k_end = unsafe { &kernel_end as *const u8 as usize };

    // 2. Initialize Physical Frame Allocator
    pmm::init(32 * 1024 * 1024, k_start, k_end);

    // 3. Setup and initialize 4 MB Kernel Heap
    let heap_size = 4 * 1024 * 1024;
    let heap_start = (k_end + 4095) & !4095;
    unsafe { ALLOCATOR.init(heap_start, heap_size); }

    // 4. Activate Virtual Memory Manager & Paging
    vmm::init();

    // 5. Setup display and input boundaries
    framebuffer::init();
    if framebuffer::is_active() {
        mouse::set_bounds(framebuffer::width(), framebuffer::height());
    } else {
        mouse::set_bounds(80, 25);
    }

    // 6. Initialize Syscalls, Filesystem, and Networking
    syscall::init();
    vfs::init();
    net::init();

    // 7. Automated diagnostics and self-test
    vmm::test_vmm();

    // 8. Start interactive console
    shell::run_shell();
}
```

## Custom Panic Handler

In a bare-metal `#![no_std]` environment, unrecoverable runtime errors invoke a custom `#[panic_handler]`:

```rust
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::set_color(Color::White, Color::Red);
    println!("\n[KERNEL PANIC]");
    if let Some(location) = info.location() {
        println!("Location: {}:{}", location.file(), location.line());
    }
    println!("Message: {}", info.message());
    println!("System halted. Please reboot.");

    logln!("[Nyxara Kernel Panic] {}", info.message());

    loop {
        unsafe { core::arch::asm!("cli; hlt"); }
    }
}
```

### Features of the Panic Handler
- **Visual Alert**: Immediately paints the screen with high-contrast White on Red text.
- **Source Inspection**: Extracts and prints the offending source file name and line number using `info.location()`.
- **Dual Logging**: Transmits the complete panic diagnostics string over COM1 serial (`logln!`) for post-mortem analysis.
- **Safe Halting**: Disables interrupts (`cli`) and enters an infinite CPU halt loop (`hlt`), preventing memory corruption.

## Freestanding Runtime Hooks

To compile without the Rust standard library:
- **Personality Routine**: `rust_eh_personality()` provides an empty stub since exception unwinding is disabled (`panic = "abort"`).
- **Unwind Resume**: `_Unwind_Resume()` halts the CPU if unexpected unwinding is triggered.
- **Global Allocator**: `#[global_allocator]` binds the kernel's custom `KernelAllocator` to the Rust standard `alloc` crate.
