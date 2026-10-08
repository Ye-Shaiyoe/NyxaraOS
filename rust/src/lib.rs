#![no_std]
#![no_main]

extern crate alloc;

pub mod commands;
pub mod cpu;
pub mod disk;
pub mod editor;
pub mod elf;
pub mod fd;
pub mod font;
pub mod framebuffer;
pub mod heap;
pub mod initrd;
pub mod line_editor;
pub mod mouse;
pub mod net;
#[path = "../../fs/nyxfs/mod.rs"]
pub mod nyxfs;
pub mod pmm;
pub mod process;
pub mod serial;
pub mod shell;
pub mod syscall;
pub mod vfs;
pub mod vga;
pub mod vmm;

use core::panic::PanicInfo;
use vga::Color;

extern "C" {
    static kernel_start: u8;
    static kernel_end: u8;
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::set_color(Color::White, Color::Red);
    println!("\n[KERNEL PANIC]");
    if let Some(location) = info.location() {
        println!("Location: {}:{}", location.file(), location.line());
    }
    println!("Message: {}", info.message());
    println!("System halted. Please reboot.");

    logln!("[Nyxara Kernel Is Panic] {}", info.message());

    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

#[no_mangle]
pub extern "C" fn nyxara_rust_main() -> ! {
    vga::clear_screen();

    let k_start = unsafe { &kernel_start as *const u8 as usize };
    let k_end = unsafe { &kernel_end as *const u8 as usize };

    pmm::init(32 * 1024 * 1024, k_start, k_end);

    // Reserve extended memory for VMM Page Tables & Kernel Stack (0x00100000..0x00140000)
    for page in (0x100000 / pmm::PAGE_SIZE)..(0x140000 / pmm::PAGE_SIZE) {
        pmm::reserve_frame(page * pmm::PAGE_SIZE);
    }

    // Heap must start ABOVE all reserved low memory regions:
    // - VMM Page Tables: 0x100000..0x120000
    // - Kernel Stack:    0x130000..0x140000
    // Safe heap base: 0x200000 (2 MB mark)
    let heap_base: usize = 0x200000;
    let heap_start = heap_base.max((k_end + 4095) & !4095);
    let heap_size = 6 * 1024 * 1024; // 6 MB heap
    let heap_end = heap_start + heap_size;

    let heap_start_page = heap_start / pmm::PAGE_SIZE;
    let heap_end_page = heap_end / pmm::PAGE_SIZE;
    for page in heap_start_page..heap_end_page {
        pmm::reserve_frame(page * pmm::PAGE_SIZE);
    }

    heap::init(heap_start, heap_size);

    vmm::init();
    framebuffer::init();
    if framebuffer::is_active() {
        mouse::set_bounds(framebuffer::width(), framebuffer::height());
    } else {
        mouse::set_bounds(80, 25);
    }
    // Clear screen again after VMM+FB init to erase any VBE boot artifacts / rainbow glitches
    vga::clear_screen();
    logln!("[Rust] Initializing syscall layer...");
    syscall::init();
    logln!("[Rust] Detecting ATA storage...");
    disk::init();
    if disk::is_available() {
        logln!("[Rust] Mounting NyxFS persistent filesystem...");
        nyxfs::init(0);
    }
    logln!("[Rust] Initializing VFS...");
    vfs::init();
    logln!("[Rust] Registering embedded userland programs...");
    initrd::init();
    logln!("[Rust] Initializing network stack...");
    net::init();
    logln!("[Rust] Core subsystems initialized.");

    print_colored!(
        Color::LightCyan,
        Color::Black,
        r#"
    _   _                            ____   _____
   | \ | |                          / __ \ / ____|
   |  \| |_   ___  ____ _ _ __ __ _| |  | | (___
   | . ` | | | \ \/ / _` | '__/ _` | |  | |\___ \
   | |\  | |_| |>  < (_| | | | (_| | |__| |____) |
   |_| \_|\__, /_/\_\__,_|_|  \__,_|\____/|_____/
           __/ |
          |___/
    "#
    );

    print_colored!(
        Color::Yellow,
        Color::Black,
        " Nyxara Operating System - Unix-like Hybrid Architecture\n"
    );
    println!(" -------------------------------------------------------------");
    print_colored!(
        Color::LightGray,
        Color::Black,
        " * Low-Level HAL & Drivers : C / Assembly (NASM)\n"
    );
    print_colored!(
        Color::LightGray,
        Color::Black,
        " * Kernel Core & Shell     : Rust (no_std, alloc)\n"
    );
    print_colored!(
        Color::LightGray,
        Color::Black,
        " * Target Architecture     : x86 (32-bit Protected Mode)\n"
    );
    println!(" -------------------------------------------------------------\n");

    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("Physical memory and kernel heap initialized.");
    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("Virtual Memory Manager (x86 Paging & Page Fault) enabled.");
    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("Unix System Calls (int 0x80) and VFS initialized.");
    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("Hardware components & RTL8139 network initialized.");
    print_colored!(Color::LightCyan, Color::Black, "[INFO] ");
    println!("Type 'help' for available commands or 'about' for details.\n");

    logln!("[Nyxara Kernel] Rust core initialized.");
    crate::serial::log_mem(pmm::total_memory() / 1024, pmm::free_memory() / 1024);

    let test_msg = "POSIX syscall test verified at boot.\n";
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 4u32, // SYS_WRITE
            in("ebx") 1u32, // fd 1
            in("ecx") test_msg.as_ptr() as u32,
            in("edx") test_msg.len() as u32,
        );
    }

    // Run automated self-test of Virtual Memory Manager (Identity & Demand Paging)
    vmm::test_vmm();

    process::init();
    shell::run_shell();
}
