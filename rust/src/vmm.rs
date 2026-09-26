// Nyxara OS - Virtual Memory Manager (VMM) & x86 Paging
// Two-level Paging (Page Directory & Page Tables) with ISR 14 Page Fault Handler

use crate::syscall::{isr_register_handler, Registers};
use crate::vga::{self, Color};

// Paging Entry Flags (x86 Standard)
pub const PAGE_PRESENT: u32 = 1 << 0; // Page is currently loaded in memory
pub const PAGE_WRITABLE: u32 = 1 << 1; // 1 = Read/Write, 0 = Read-Only
pub const PAGE_USER: u32 = 1 << 2; // 1 = User mode (Ring 3), 0 = Supervisor (Ring 0)
pub const PAGE_WRITETHROUGH: u32 = 1 << 3; // 1 = Write-through caching
pub const PAGE_NOCACHE: u32 = 1 << 4; // 1 = Cache disabled
pub const PAGE_ACCESSED: u32 = 1 << 5; // 1 = CPU has accessed this page
pub const PAGE_DIRTY: u32 = 1 << 6; // 1 = CPU has written to this page
pub const PAGE_FRAME_MASK: u32 = 0xFFFFF000; // Physical frame address (4KB aligned)

pub const PAGE_SIZE: usize = 4096;
pub const ENTRIES_PER_TABLE: usize = 1024;

// Total memory identity mapped on boot: 64 MB (16 tables * 4MB per table)
pub const NUM_IDENTITY_TABLES: usize = 16;
pub const IDENTITY_MAPPED_SIZE: usize = NUM_IDENTITY_TABLES * ENTRIES_PER_TABLE * PAGE_SIZE;

// Dedicated Demand Paging Range: 0xC0000000 to 0xC1000000 (16 MB)
pub const DEMAND_PAGING_START: usize = 0xC0000000;
pub const DEMAND_PAGING_END: usize = 0xC1000000;

// ------------------------------------------------------------------------------
// Page Directory and Page Table Structures (Aligned to 4096 bytes)
// ------------------------------------------------------------------------------
#[derive(Copy, Clone)]
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u32; ENTRIES_PER_TABLE],
}

#[derive(Copy, Clone)]
#[repr(C, align(4096))]
pub struct PageDirectory {
    pub entries: [u32; ENTRIES_PER_TABLE],
}

// Page Directory and Page Tables located in extended memory (1 MB+ safe DRAM)
pub const PAGE_DIR_PHYS: usize = 0x00100000; // 4 KB Page Directory
pub const LFB_TABLE_PHYS: usize = 0x00101000; // 4 KB LFB Page Table
pub const IDENT_TABLES_PHYS: usize = 0x00110000; // 64 KB (16 Page Tables)

#[inline]
pub unsafe fn get_page_directory() -> &'static mut PageDirectory {
    &mut *(PAGE_DIR_PHYS as *mut PageDirectory)
}

extern "C" {
    fn fb_is_active() -> u8;
    fn fb_get_info() -> *const BootInfoVmm;
}

#[repr(C)]
struct BootInfoVmm {
    magic: u32,
    fb_addr: u32,
    fb_width: u16,
    fb_height: u16,
    fb_pitch: u16,
    fb_bpp: u8,
    is_graphical: u8,
    reserved: [u8; 2],
}

// LFB fallback is 0xFD000000 (PD index 1012).
pub const LFB_PHYS_BASE: usize = 0xFD000000;

static mut PAGING_ENABLED: bool = false;
static mut DEMAND_PAGE_FAULTS: usize = 0;
static mut TOTAL_MAPPED_PAGES: usize = 0;

// ------------------------------------------------------------------------------
// Low-Level x86 Control Register Access
// ------------------------------------------------------------------------------

#[inline]
pub unsafe fn load_cr3(pd_phys_addr: usize) {
    core::arch::asm!(
        "mov cr3, {0}",
        in(reg) pd_phys_addr,
        options(nostack)
    );
}

#[inline]
pub unsafe fn read_cr3() -> usize {
    let val: usize;
    core::arch::asm!(
        "mov {0}, cr3",
        out(reg) val,
        options(nostack)
    );
    val
}

#[inline]
pub unsafe fn read_cr2() -> usize {
    let val: usize;
    core::arch::asm!(
        "mov {0}, cr2",
        out(reg) val,
        options(nostack)
    );
    val
}

#[inline]
pub fn invalidate_tlb(addr: usize) {
    unsafe {
        core::arch::asm!(
            "invlpg [{0}]",
            in(reg) addr,
            options(nostack, preserves_flags)
        );
    }
}

// ------------------------------------------------------------------------------
// VMM Initialization
// ------------------------------------------------------------------------------

pub fn init() {
    crate::logln!("[VMM] Initializing Virtual Memory Manager (Two-level x86 Paging)...");

    unsafe {
        // Zero out the page directory, identity tables, and LFB table in extended memory
        core::ptr::write_bytes(PAGE_DIR_PHYS as *mut u8, 0, PAGE_SIZE);
        core::ptr::write_bytes(LFB_TABLE_PHYS as *mut u8, 0, PAGE_SIZE);
        core::ptr::write_bytes(
            IDENT_TABLES_PHYS as *mut u8,
            0,
            NUM_IDENTITY_TABLES * PAGE_SIZE,
        );

        let pd = get_page_directory();
        let ident_tables = &mut *(IDENT_TABLES_PHYS as *mut [PageTable; NUM_IDENTITY_TABLES]);
        let lfb_table = &mut *(LFB_TABLE_PHYS as *mut PageTable);

        // 1. Identity map the first 64 MB of physical memory
        for t in 0..NUM_IDENTITY_TABLES {
            let base_phys = t * ENTRIES_PER_TABLE * PAGE_SIZE;
            for i in 0..ENTRIES_PER_TABLE {
                let page_phys = base_phys + (i * PAGE_SIZE);
                ident_tables[t].entries[i] = (page_phys as u32) | PAGE_PRESENT | PAGE_WRITABLE;
            }

            // Link Page Directory entry t to IDENT_TABLES[t]
            let table_phys = IDENT_TABLES_PHYS + (t * PAGE_SIZE);
            pd.entries[t] = (table_phys as u32) | PAGE_PRESENT | PAGE_WRITABLE;
        }

        TOTAL_MAPPED_PAGES = NUM_IDENTITY_TABLES * ENTRIES_PER_TABLE;

        // Map the VBE LFB (4MB slot at physical fb_addr).
        let lfb_phys = if fb_is_active() != 0 {
            let info = &*fb_get_info();
            (info.fb_addr as usize) & !0x3FFFFF // 4MB aligned boundary
        } else {
            LFB_PHYS_BASE
        };
        let lfb_pd_idx = (lfb_phys >> 22) & 0x3FF;

        for i in 0..ENTRIES_PER_TABLE {
            let page_phys = lfb_phys + i * PAGE_SIZE;
            lfb_table.entries[i] = (page_phys as u32) | PAGE_PRESENT | PAGE_WRITABLE | PAGE_NOCACHE;
        }
        pd.entries[lfb_pd_idx] = (LFB_TABLE_PHYS as u32) | PAGE_PRESENT | PAGE_WRITABLE;
        TOTAL_MAPPED_PAGES += ENTRIES_PER_TABLE;

        // Clear remaining directory entries (unmapped / not present)
        for t in NUM_IDENTITY_TABLES..ENTRIES_PER_TABLE {
            if t != lfb_pd_idx {
                pd.entries[t] = 0;
            }
        }

        // 2. Register ISR 14 (Page Fault Exception Handler)
        isr_register_handler(14, page_fault_handler);
        crate::logln!("[VMM] ISR 14 Page Fault Exception Handler registered.");

        // 3. Load CR3 with Page Directory physical address
        load_cr3(PAGE_DIR_PHYS);
        crate::logln!(
            "[VMM] Page Directory loaded into CR3 at 0x{:08X}",
            PAGE_DIR_PHYS
        );

        // 4. Enable Paging (PG bit 31) and Write Protect (WP bit 16) in CR0
        let mut cr0: u32;
        core::arch::asm!("mov {0}, cr0", out(reg) cr0, options(nostack));
        cr0 |= 0x80000000; // Bit 31: PG (Paging Enable)
        cr0 |= 0x00010000; // Bit 16: WP (Write Protect for Supervisor)
        core::arch::asm!("mov cr0, {0}", in(reg) cr0, options(nostack));

        PAGING_ENABLED = true;
    }

    crate::logln!(
        "[VMM] x86 Paging successfully enabled (64MB Identity Mapped, CR0.PG=1, CR0.WP=1)."
    );
}

// ------------------------------------------------------------------------------
// Dynamic Virtual Memory Mapping API
// ------------------------------------------------------------------------------

/// Map a 4KB virtual address to a physical address with specified flags.
pub fn map_page(virt_addr: usize, phys_addr: usize, flags: u32) -> Result<(), &'static str> {
    unsafe {
        let pd_idx = (virt_addr >> 22) & 0x3FF;
        let pt_idx = (virt_addr >> 12) & 0x3FF;

        let pd = get_page_directory();
        let pde = pd.entries[pd_idx];
        let pt_phys = if (pde & PAGE_PRESENT) == 0 {
            // Allocate a new frame from PMM for the Page Table
            let new_table_frame = match crate::pmm::alloc_frame() {
                Some(f) => f,
                None => return Err("Out of physical memory to allocate Page Table"),
            };

            // Zero the new table
            core::ptr::write_bytes(new_table_frame as *mut u8, 0, PAGE_SIZE);

            // Set PDE
            let pde_val =
                (new_table_frame as u32) | PAGE_PRESENT | PAGE_WRITABLE | (flags & PAGE_USER);
            pd.entries[pd_idx] = pde_val;
            new_table_frame
        } else {
            if (flags & PAGE_USER) != 0 {
                pd.entries[pd_idx] |= PAGE_USER;
            }
            (pde & PAGE_FRAME_MASK) as usize
        };

        let pt = &mut *(pt_phys as *mut PageTable);
        let old_entry = pt.entries[pt_idx];
        pt.entries[pt_idx] = (phys_addr as u32 & PAGE_FRAME_MASK) | (flags & 0xFFF) | PAGE_PRESENT;

        if (old_entry & PAGE_PRESENT) == 0 {
            TOTAL_MAPPED_PAGES += 1;
        }

        invalidate_tlb(virt_addr);
        Ok(())
    }
}

/// Unmap a virtual page and invalidate TLB cache.
pub fn unmap_page(virt_addr: usize) -> Result<(), &'static str> {
    unsafe {
        let pd_idx = (virt_addr >> 22) & 0x3FF;
        let pt_idx = (virt_addr >> 12) & 0x3FF;

        let pd = get_page_directory();
        let pde = pd.entries[pd_idx];
        if (pde & PAGE_PRESENT) == 0 {
            return Err("Page table not present");
        }

        let pt_phys = (pde & PAGE_FRAME_MASK) as usize;
        let pt = &mut *(pt_phys as *mut PageTable);

        if (pt.entries[pt_idx] & PAGE_PRESENT) != 0 {
            pt.entries[pt_idx] = 0;
            if TOTAL_MAPPED_PAGES > 0 {
                TOTAL_MAPPED_PAGES -= 1;
            }
        }

        invalidate_tlb(virt_addr);
        Ok(())
    }
}

/// Translate a virtual address to physical address, if mapped.
pub fn get_phys_addr(virt_addr: usize) -> Option<usize> {
    unsafe {
        let pd_idx = (virt_addr >> 22) & 0x3FF;
        let pt_idx = (virt_addr >> 12) & 0x3FF;
        let offset = virt_addr & (PAGE_SIZE - 1);

        let pd = get_page_directory();
        let pde = pd.entries[pd_idx];
        if (pde & PAGE_PRESENT) == 0 {
            return None;
        }

        let pt_phys = (pde & PAGE_FRAME_MASK) as usize;
        let pt = &*(pt_phys as *const PageTable);

        let pte = pt.entries[pt_idx];
        if (pte & PAGE_PRESENT) == 0 {
            return None;
        }

        let frame = (pte & PAGE_FRAME_MASK) as usize;
        Some(frame + offset)
    }
}

pub fn is_paging_enabled() -> bool {
    unsafe { PAGING_ENABLED }
}

pub fn total_mapped_pages() -> usize {
    unsafe { TOTAL_MAPPED_PAGES }
}

pub fn demand_page_fault_count() -> usize {
    unsafe { DEMAND_PAGE_FAULTS }
}

// ------------------------------------------------------------------------------
// ISR 14: Page Fault Exception Handler & Demand Paging Engine
// ------------------------------------------------------------------------------

extern "C" fn page_fault_handler(regs: &mut Registers) {
    let fault_addr = unsafe { read_cr2() };
    let err = regs.err_code;

    let present = (err & 0x01) != 0; // 0: page not present, 1: protection violation
    let write = (err & 0x02) != 0; // 0: read access, 1: write access
    let user = (err & 0x04) != 0; // 0: supervisor (Ring 0), 1: user (Ring 3)
    let rsvd = (err & 0x08) != 0; // 1: reserved bit set in page table entry
    let ifetch = (err & 0x10) != 0; // 1: instruction fetch

    // --------------------------------------------------------------------------
    // Demand Paging Zone (0xC0000000 - 0xC1000000)
    // --------------------------------------------------------------------------
    if fault_addr >= DEMAND_PAGING_START && fault_addr < DEMAND_PAGING_END && !present {
        if let Some(frame) = crate::pmm::alloc_frame() {
            // Clear the frame content
            unsafe {
                core::ptr::write_bytes(frame as *mut u8, 0, PAGE_SIZE);
            }

            let page_aligned = fault_addr & !(PAGE_SIZE - 1);
            if map_page(page_aligned, frame, PAGE_WRITABLE).is_ok() {
                unsafe {
                    DEMAND_PAGE_FAULTS += 1;
                }
                crate::logln!(
                    "[VMM] Demand Paging Handled: virt 0x{:08X} -> phys 0x{:08X} (Total: {})",
                    page_aligned,
                    frame,
                    unsafe { DEMAND_PAGE_FAULTS }
                );
                return; // Return to retry instruction with newly mapped memory
            }
        }
    }

    // --------------------------------------------------------------------------
    // Unhandled / Illegal Page Fault -> Kernel Panic Diagnostic Screen
    // --------------------------------------------------------------------------
    vga::set_color(Color::White, Color::Red);
    crate::println!("\n========================================================");
    crate::println!("       [KERNEL PANIC: PAGE FAULT EXCEPTION - ISR 14]     ");
    crate::println!("========================================================");

    vga::set_color(Color::Yellow, Color::Red);
    crate::print!(" Faulting Linear Address (CR2) : ");
    vga::set_color(Color::White, Color::Red);
    crate::println!("0x{:08X}", fault_addr);

    vga::set_color(Color::Yellow, Color::Red);
    crate::print!(" Instruction Pointer (EIP)     : ");
    vga::set_color(Color::White, Color::Red);
    crate::println!("0x{:08X}", regs.eip);

    vga::set_color(Color::Yellow, Color::Red);
    crate::print!(" Error Code Value              : ");
    vga::set_color(Color::White, Color::Red);
    crate::println!("0x{:08X} ({})", err, err);

    vga::set_color(Color::LightGray, Color::Red);
    crate::println!(" Breakdown:");
    crate::println!(
        "   - Condition : {}",
        if present {
            "Page-Level Protection Violation"
        } else {
            "Page Not Present (Unmapped/Null)"
        }
    );
    crate::println!(
        "   - Operation : {}",
        if write { "Write Access" } else { "Read Access" }
    );
    crate::println!(
        "   - Mode      : {}",
        if user {
            "User Mode (Ring 3)"
        } else {
            "Kernel Mode (Ring 0 / Supervisor)"
        }
    );
    if rsvd {
        crate::println!("   - Reserved  : Reserved bit set in page table entry!");
    }
    if ifetch {
        crate::println!("   - Cause     : Instruction Fetch");
    }

    vga::set_color(Color::LightCyan, Color::Red);
    crate::println!(
        " Registers: EAX=0x{:08X} EBX=0x{:08X} ECX=0x{:08X} EDX=0x{:08X}",
        regs.eax,
        regs.ebx,
        regs.ecx,
        regs.edx
    );
    crate::println!(
        "            ESP=0x{:08X} EBP=0x{:08X} ESI=0x{:08X} EDI=0x{:08X}",
        regs.esp,
        regs.ebp,
        regs.esi,
        regs.edi
    );

    vga::set_color(Color::White, Color::Red);
    crate::println!("\n System halted for security. Please reboot or inspect memory.");
    crate::println!("========================================================");

    crate::logln!(
        "[CRITICAL] Page Fault Panic! CR2=0x{:08X}, EIP=0x{:08X}, ErrCode=0x{:08X}",
        fault_addr,
        regs.eip,
        err
    );

    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

// ------------------------------------------------------------------------------
// Automated VMM Verification Routine
// ------------------------------------------------------------------------------

pub fn test_vmm() -> bool {
    crate::logln!("[VMM Test] Starting Virtual Memory Manager test suite...");

    // Test 1: Identity Mapping check
    let phys_kstart = get_phys_addr(0x10000);
    if phys_kstart != Some(0x10000) {
        crate::logln!("[VMM Test] Identity mapping check FAILED at 0x10000");
        return false;
    }

    let phys_vga = get_phys_addr(0xB8000);
    if phys_vga != Some(0xB8000) {
        crate::logln!("[VMM Test] Identity mapping check FAILED at 0xB8000");
        return false;
    }
    crate::logln!("[VMM Test] Step 1: Kernel & VGA identity mapping verified.");

    // Test 2: Demand Paging transparent allocation test
    let test_demand_addr = 0xC0002000 as *mut u32;
    let test_val: u32 = 0x5A5A1234;

    // Before writing, it should not be mapped
    let before = get_phys_addr(0xC0002000);
    if before.is_some() {
        crate::logln!("[VMM Test] 0xC0002000 was unexpectedly already mapped");
        return false;
    }

    // Write to unmapped address in demand zone -> triggers ISR 14 -> auto-allocates -> succeeds
    unsafe {
        core::ptr::write_volatile(test_demand_addr, test_val);
    }

    // Read back value
    let read_back = unsafe { core::ptr::read_volatile(test_demand_addr) };
    if read_back != test_val {
        crate::logln!(
            "[VMM Test] Demand paging readback mismatch: got 0x{:08X}, expected 0x{:08X}",
            read_back,
            test_val
        );
        return false;
    }

    // Verify it is now mapped to a physical frame
    let after = get_phys_addr(0xC0002000);
    if after.is_none() {
        crate::logln!("[VMM Test] Demand paging physical mapping resolution failed");
        return false;
    }
    crate::logln!(
        "[VMM Test] Step 2: Demand paging auto-allocation verified (phys=0x{:08X}).",
        after.unwrap()
    );

    // Test 3: Manual page mapping & unmapping
    let custom_virt = 0xD0001000;
    if let Some(frame) = crate::pmm::alloc_frame() {
        if map_page(custom_virt, frame, PAGE_WRITABLE).is_err() {
            crate::logln!("[VMM Test] Failed to map custom page");
            return false;
        }

        let custom_ptr = custom_virt as *mut u32;
        unsafe {
            core::ptr::write_volatile(custom_ptr, 0xDEADCAFE);
            if core::ptr::read_volatile(custom_ptr) != 0xDEADCAFE {
                return false;
            }
        }

        if unmap_page(custom_virt).is_err() {
            crate::logln!("[VMM Test] Failed to unmap custom page");
            return false;
        }

        crate::pmm::free_frame(frame);
        crate::logln!("[VMM Test] Step 3: Dynamic map & unmap verified.");
    } else {
        crate::logln!("[VMM Test] Could not allocate frame for step 3");
        return false;
    }

    crate::logln!("[VMM Test] All VMM tests PASSED!");
    true
}
