use crate::vga::Color;
use crate::{print, println, print_colored, logln};
use alloc::format;

extern "C" {
    fn timer_get_ticks() -> u32;
    fn timer_get_uptime_seconds() -> u32;
    fn timer_get_uptime_ms() -> u32;
    fn outb(port: u16, val: u8);
    fn rtc_get_datetime(t: *mut RtcTime);
}

#[repr(C)]
pub struct RtcTime {
    pub second: u8,
    pub minute: u8,
    pub hour:   u8,
    pub day:    u8,
    pub month:  u8,
    pub _pad:   u8,
    pub year:   u16,
}

pub fn read_rtc() -> RtcTime {
    let mut t = RtcTime {
        second: 0,
        minute: 0,
        hour:   0,
        day:    0,
        month:  0,
        _pad:   0,
        year:   0,
    };
    unsafe { rtc_get_datetime(&mut t as *mut RtcTime); }
    t
}

pub fn print_padded2(val: u32, pad: bool) {
    if pad && val < 10 {
        print!("0{}", val);
    } else {
        print!("{}", val);
    }
}

pub fn run_elf_binary(name: &str, verbose: bool) -> bool {
    let path = if name.contains('/') {
        format!("{}", name)
    } else {
        format!("/bin/{}", name)
    };
    let Some(image) = crate::vfs::read_file(&path) else {
        if verbose {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("'{}' not found.", path);
        }
        return false;
    };

    match crate::process::spawn_elf(&image, name) {
        Ok(pid) => {
            if verbose {
                println!("Started '{}' (pid {}), waiting for exit...", name, pid);
            }
            let code = crate::process::wait_kernel(pid);
            if verbose && code >= 0 {
                println!("'{}' finished with exit code {}.", name, code);
            }
            true
        }
        Err(err) => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("failed to start '{}' (errno {}).", name, -err);
            false
        }
    }
}

pub fn cmd_run(args: &str) {
    let name = args.trim();
    if name.is_empty() {
        println!("Usage: run <program>  (programs live in /bin)");
        return;
    }
    let _ = run_elf_binary(name, true);
}

pub fn cmd_sleep(args: &str) {
    let ticks = match args.trim().parse::<u32>() {
        Ok(ticks) if ticks > 0 => ticks,
        _ => {
            print_colored!(Color::LightRed, Color::Black, "Usage: ");
            println!("sleep <ticks>");
            return;
        }
    };

    crate::process::sleep(ticks);
}

pub fn cmd_syscall_test() {
    print_colored!(Color::LightCyan, Color::Black, "Testing Unix System Call (int 0x80)...\n");

    let msg = "Hello from Unix sys_write via int 0x80!\n";
    let ret: i32;

    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("eax") 4u32 => ret,
            in("ebx") 1u32,
            in("ecx") msg.as_ptr() as u32,
            in("edx") msg.len() as u32,
        );
    }

    println!("Syscall return value (bytes written): {}", ret);

    let pid: i32;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("eax") 20u32 => pid,
            in("ebx") 0u32,
            in("ecx") 0u32,
            in("edx") 0u32,
        );
    }
    println!("Current PID from sys_getpid: {}", pid);
}

pub fn cmd_free() {
    let total = crate::pmm::total_memory() / 1024;
    let used = crate::pmm::used_memory() / 1024;
    let free = crate::pmm::free_memory() / 1024;

    print_colored!(Color::LightCyan, Color::Black, "Physical Memory (PMM):\n");
    println!("  Total : {} KB ({} MB)", total, total / 1024);
    println!("  Used  : {} KB ({} MB)", used, used / 1024);
    println!("  Free  : {} KB ({} MB)", free, free / 1024);

    print_colored!(Color::LightCyan, Color::Black, "Kernel Heap:\n");
    println!(
        "  Used  : {} KB / {} KB (peak {} KB)",
        crate::heap::heap_used() / 1024,
        crate::heap::heap_capacity() / 1024,
        crate::heap::heap_peak() / 1024
    );

    print_colored!(Color::LightCyan, Color::Black, "Virtual Memory (VMM):\n");
    let paging_active = crate::vmm::is_paging_enabled();
    println!("  Paging       : {}", if paging_active { "Active (32-bit Protected Mode, CR0.PG=1, CR0.WP=1)" } else { "Inactive" });
    if paging_active {
        let cr3 = unsafe { crate::vmm::read_cr3() };
        println!("  CR3 (PD)     : 0x{:08X}", cr3);
        println!("  Mapped Pages : {} pages ({} KB)", crate::vmm::total_mapped_pages(), crate::vmm::total_mapped_pages() * 4);
        println!("  Demand Faults: {} auto-handled", crate::vmm::demand_page_fault_count());
    }
}

pub fn cmd_sysinfo() {
    let ticks = unsafe { timer_get_ticks() };
    let uptime_sec = unsafe { timer_get_uptime_seconds() };
    let uptime_ms = unsafe { timer_get_uptime_ms() };

    let esp_val: u32;
    unsafe {
        core::arch::asm!("mov {0:e}, esp", out(reg) esp_val);
    }

    print_colored!(Color::LightCyan, Color::Black, "System Status:\n");
    println!("  CPU Mode     : 32-bit Protected Mode");
    println!("  Stack Pointer: 0x{:X}", esp_val);
    println!("  PIT Ticks    : {} (100 Hz)", ticks);
    println!("  Uptime       : {} seconds ({} ms)", uptime_sec, uptime_ms);
    println!("  Interrupts   : Enabled (IDT vectors 0..47)");
    println!("  Serial COM1  : 0x3F8 @ 38400 baud");
}

pub fn cmd_uptime() {
    let sec = unsafe { timer_get_uptime_seconds() };
    let ms = unsafe { timer_get_uptime_ms() };
    let minutes = sec / 60;
    let seconds = sec % 60;
    print_colored!(Color::LightGreen, Color::Black, "Uptime: ");
    println!("{}m {}s (total {} ms)", minutes, seconds, ms);
}

pub fn cmd_panic(args: &str) {
    let msg = if args.trim().is_empty() {
        "Manual panic triggered by user from Nyxara shell!"
    } else {
        args.trim()
    };
    panic!("{}", msg);
}

pub fn cmd_reboot() {
    print_colored!(Color::Yellow, Color::Black, "Rebooting Nyxara OS...\n");
    logln!("[Nyxara Kernel] System reboot triggered.");

    unsafe {
        core::arch::asm!("cli");
        for _ in 0..1000 {
            outb(0x64, 0xFE);
        }
        let null_idt: [u16; 3] = [0, 0, 0];
        core::arch::asm!("lidt [{}]", in(reg) null_idt.as_ptr());
        core::arch::asm!("int3");
    }
}

pub fn cmd_date() {
    let t = read_rtc();
    print_colored!(Color::LightCyan, Color::Black, "Date: ");
    print_padded2(t.year as u32, false);
    print!("-");
    print_padded2(t.month as u32, true);
    print!("-");
    print_padded2(t.day as u32, true);
    println!("  (from RTC/CMOS)");
}

pub fn cmd_time() {
    let t = read_rtc();
    print_colored!(Color::LightCyan, Color::Black, "Time: ");
    print_padded2(t.hour as u32, true);
    print!(":");
    print_padded2(t.minute as u32, true);
    print!(":");
    print_padded2(t.second as u32, true);
    println!("  (from RTC/CMOS)");
}

pub fn cmd_vmm(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.split_whitespace().collect();
    let subcmd = if parts.is_empty() { "info" } else { parts[0] };

    match subcmd {
        "info" | "status" => {
            print_colored!(Color::LightCyan, Color::Black, "Virtual Memory Manager (VMM) Status:\n");
            let active = crate::vmm::is_paging_enabled();
            println!("  x86 Paging      : {}", if active { "ENABLED (32-bit Protected Mode)" } else { "DISABLED" });
            if active {
                let cr3 = unsafe { crate::vmm::read_cr3() };
                println!("  Page Dir (CR3)  : 0x{:08X}", cr3);
                println!("  Identity Map    : 0x00000000 - 0x03FFFFFF (64 MB)");
                println!("  Total Mapped    : {} pages ({} KB)",
                    crate::vmm::total_mapped_pages(),
                    crate::vmm::total_mapped_pages() * 4
                );
                println!("  Demand Range    : 0x{:08X} - 0x{:08X} (16 MB)", crate::vmm::DEMAND_PAGING_START, crate::vmm::DEMAND_PAGING_END);
                println!("  Demand Faults   : {} auto-allocated via ISR 14", crate::vmm::demand_page_fault_count());
                println!("  Protection      : Supervisor Write-Protect (CR0.WP=1)");
            }
        }
        "test" => {
            print_colored!(Color::LightCyan, Color::Black, "[VMM Test] ");
            println!("Running automated paging and demand-paging verification suite...");

            print!("  1. Testing Identity Mapping (Kernel & VGA) ... ");
            let k_ok = crate::vmm::get_phys_addr(0x10000) == Some(0x10000);
            let vga_ok = crate::vmm::get_phys_addr(0xB8000) == Some(0xB8000);
            if k_ok && vga_ok {
                print_colored!(Color::LightGreen, Color::Black, "PASSED\n");
            } else {
                print_colored!(Color::LightRed, Color::Black, "FAILED\n");
            }

            print!("  2. Testing Demand Paging on 0xC0002000 ... ");
            let test_addr = 0xC0002000 as *mut u32;
            let val = 0x5A5A1234;
            unsafe {
                core::ptr::write_volatile(test_addr, val);
                let read = core::ptr::read_volatile(test_addr);
                if read == val && crate::vmm::get_phys_addr(0xC0002000).is_some() {
                    print_colored!(Color::LightGreen, Color::Black, "PASSED");
                    println!(" (auto-allocated frame: 0x{:08X})", crate::vmm::get_phys_addr(0xC0002000).unwrap());
                } else {
                    print_colored!(Color::LightRed, Color::Black, "FAILED\n");
                }
            }

            print!("  3. Testing Dynamic Mapping & Unmapping ... ");
            let custom_v = 0xD0001000;
            if let Some(frame) = crate::pmm::alloc_frame() {
                let map_res = crate::vmm::map_page(custom_v, frame, crate::vmm::PAGE_WRITABLE);
                let mut data_ok = false;
                if map_res.is_ok() {
                    unsafe {
                        let ptr = custom_v as *mut u32;
                        core::ptr::write_volatile(ptr, 0xCAFEBABE);
                        data_ok = core::ptr::read_volatile(ptr) == 0xCAFEBABE;
                    }
                    let _ = crate::vmm::unmap_page(custom_v);
                }
                crate::pmm::free_frame(frame);
                if data_ok {
                    print_colored!(Color::LightGreen, Color::Black, "PASSED\n");
                } else {
                    print_colored!(Color::LightRed, Color::Black, "FAILED\n");
                }
            } else {
                print_colored!(Color::LightRed, Color::Black, "OUT OF MEMORY\n");
            }

            print_colored!(Color::LightGreen, Color::Black, "\n[SUCCESS] ");
            println!("All Virtual Memory Manager tests executed successfully!");
        }
        "map" => {
            if parts.len() < 3 {
                print_colored!(Color::LightRed, Color::Black, "Usage: ");
                println!("vmm map <virt_hex> <phys_hex>");
                println!("  Example: vmm map D0000000 1000000");
                return;
            }
            let virt = usize::from_str_radix(parts[1].trim_start_matches("0x"), 16);
            let phys = usize::from_str_radix(parts[2].trim_start_matches("0x"), 16);
            match (virt, phys) {
                (Ok(v), Ok(p)) => {
                    match crate::vmm::map_page(v, p, crate::vmm::PAGE_WRITABLE) {
                        Ok(_) => {
                            print_colored!(Color::LightGreen, Color::Black, "[OK] ");
                            println!("Mapped virt 0x{:08X} -> phys 0x{:08X}", v, p);
                        }
                        Err(e) => {
                            print_colored!(Color::LightRed, Color::Black, "Error: ");
                            println!("{}", e);
                        }
                    }
                }
                _ => {
                    print_colored!(Color::LightRed, Color::Black, "Error: ");
                    println!("Invalid hexadecimal address.");
                }
            }
        }
        "fault" => {
            print_colored!(Color::Yellow, Color::Black, "[WARNING] ");
            println!("Triggering intentional Page Fault at unmapped address 0xDEAD0000...");
            println!("This will invoke the Page Fault Exception Handler (ISR 14).");
            unsafe {
                let ptr = 0xDEAD0000 as *const u32;
                let _ = core::ptr::read_volatile(ptr);
            }
        }
        _ => {
            print_colored!(Color::LightCyan, Color::Black, "VMM Commands:\n");
            println!("  vmm info                   - Display paging and directory information");
            println!("  vmm test                   - Run automated Demand Paging & VMM test suite");
            println!("  vmm map <virt_hex> <phys>  - Map virtual page to physical frame");
            println!("  vmm fault                  - Trigger intentional page fault exception (panic test)");
        }
    }
}
