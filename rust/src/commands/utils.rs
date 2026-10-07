use crate::vga::{self, Color};
use crate::{print, println, print_colored, logln};
use crate::{mouse, framebuffer};
use alloc::format;

extern "C" {
    fn timer_get_uptime_seconds() -> u32;
    fn keyboard_has_char() -> bool;
    fn keyboard_getchar() -> u16;
}

pub fn print_pad_right(s: &str, width: usize) {
    print!("{}", s);
    if s.len() < width {
        for _ in s.len()..width {
            print!(" ");
        }
    } else {
        print!("  ");
    }
}

pub fn cmd_help() {
    print_colored!(Color::LightCyan, Color::Black, "Commands:\n");
    println!("  help              - Display this help reference");
    println!("  clear             - Clear screen");
    println!("  about             - System information");
    println!("  sysinfo           - Display hardware and CPU status");
    println!("  free / meminfo    - Display physical memory and allocator status");
    println!("  uptime            - Display system uptime");
    println!("  ps                - Display kernel tasks");
    println!("  yield             - Voluntarily yield the CPU");
    println!("  run <program>     - Execute a userland ELF program from /bin");
    println!("  hello             - Execute NyxC hello world program");
    println!("  sleep <ticks>     - Block the current task");
    println!("  pwd               - Display current directory");
    println!("  cd <dir>          - Change current directory");
    println!("  mkdir <dir>       - Create a directory");
    println!("  rm <file>         - Remove a file");
    println!("  rmdir <dir>       - Remove an empty directory");
    println!("  cp <src> <dst>    - Copy a file");
    println!("  mv <src> <dst>    - Move or rename a file");
    println!("  stat <path>       - Display inode information");
    println!("  date              - Display current date from RTC/CMOS");
    println!("  time              - Display current time from RTC/CMOS");
    println!("  ls                - List files in virtual filesystem (VFS)");
    println!("  cat <file>        - Display contents of a file");
    println!("  touch <file>      - Create empty file");
    println!("  write <file> <tx> - Write text to a file");
    println!("  mway <file>       - Open full-screen text editor (Ctrl+S save, Ctrl+Q exit)");
    println!("  vmm [info|test]   - Virtual Memory Manager & x86 Paging status/tests");
    println!("  syscall           - Test Unix int 0x80 system call");
    println!("  echo <text>       - Print text to screen");
    println!("  color <fg> <bg>   - Change console color (0..15)");
    println!("  calc <a op b>     - Integer calculator");
    println!("  ifconfig [args]   - Display/configure network interface eth0");
    println!("  dhcp              - Obtain dynamic IP lease from DHCP server (DORA)");
    println!("  dns <host>        - Resolve domain name to IPv4 address via DNS");
    println!("  ping <host>       - Send ICMP echo requests to target host/IP");
    println!("  arp [-a|-c]       - View dynamic ARP cache or flush entries");
    println!("  netstat           - Display interface, socket, and traffic statistics");
    println!("  curl <url>        - Fetch HTTP web content over TCP");
    println!("  httpd [port]      - Run embedded Nyxara HTTP web server");
    println!("  nc [-u] <ip> <p>  - Send raw network payload via Netcat");
    println!("  panic [msg]       - Trigger Rust Kernel Panic");
    println!("  reboot            - Restart the computer");
    println!("  lalaufetch        - Neofetch-style system info with galaxy logo");
    println!("  mouse [test]      - Display PS/2 mouse status or interactive pointer test");
    println!("  paint             - Interactive mouse drawing canvas (ESC to exit)");
}

pub fn cmd_clear() {
    vga::clear_screen();
    print_colored!(Color::LightGreen, Color::Black, "Nyxara OS - Unix-like Hybrid C & Rust Operating System\n\n");
}

pub fn cmd_about() {
    println!("Architecture : x86 (32-bit Protected Mode)");
    println!("Kernel Core  : Rust (no_std, alloc, physical memory & heap)");
    println!("HAL Drivers  : C / Assembly (GDT, IDT, PIC, PIT, PS/2, UART)");
    println!("Target Model : Unix-like OS with POSIX roadmap");
}

pub fn cmd_echo(args: &str) {
    println!("{}", args);
}

pub fn cmd_color(args: &str) {
    let mut parts = args.split_whitespace();
    let fg_str = parts.next();
    let bg_str = parts.next();

    if let (Some(f), Some(b)) = (fg_str, bg_str) {
        if let (Ok(fg_num), Ok(bg_num)) = (f.parse::<u8>(), b.parse::<u8>()) {
            if fg_num < 16 && bg_num < 16 {
                vga::set_color(Color::from_u8(fg_num), Color::from_u8(bg_num));
                println!("Color updated: fg={}, bg={}", fg_num, bg_num);
                return;
            }
        }
    }

    print_colored!(Color::LightRed, Color::Black, "Usage: ");
    println!("color <fg:0-15> <bg:0-15>");
    println!("Colors: 0:Black, 1:Blue, 2:Green, 3:Cyan, 4:Red, 5:Magenta, 6:Brown, 7:LGray,");
    println!("        8:DGray, 9:LBlue, 10:LGreen, 11:LCyan, 12:LRed, 13:LMagenta, 14:Yellow, 15:White");
}

pub fn cmd_calc(args: &str) {
    let mut parts = args.split_whitespace();
    let a_str = parts.next();
    let op_str = parts.next();
    let b_str = parts.next();

    if let (Some(a_s), Some(op), Some(b_s)) = (a_str, op_str, b_str) {
        if let (Ok(a), Ok(b)) = (a_s.parse::<i32>(), b_s.parse::<i32>()) {
            let res = match op {
                "+" => Some(a.wrapping_add(b)),
                "-" => Some(a.wrapping_sub(b)),
                "*" => Some(a.wrapping_mul(b)),
                "/" => {
                    if b == 0 {
                        print_colored!(Color::LightRed, Color::Black, "Error: ");
                        println!("Division by zero!");
                        return;
                    }
                    Some(a / b)
                }
                "%" => {
                    if b == 0 {
                        print_colored!(Color::LightRed, Color::Black, "Error: ");
                        println!("Modulo by zero!");
                        return;
                    }
                    Some(a % b)
                }
                _ => None,
            };

            if let Some(val) = res {
                print_colored!(Color::LightGreen, Color::Black, "Result: ");
                println!("{} {} {} = {}", a, op, b, val);
                return;
            }
        }
    }

    print_colored!(Color::LightRed, Color::Black, "Usage: ");
    println!("calc <num1> <+|-|*|/|%> <num2> (e.g. calc 100 * 5)");
}

pub fn cmd_lalaufetch() {
    use crate::framebuffer::palette;

    vga::clear_screen();

    let logo: &[&str] = &[
        "           .   *   .      .       ",
        "         ....oooooooooo..         ",
        "       ..ooo@@@@@@@@@@@@@@oo.     ",
        "     ..oo@@@@@oooooo@@@@@@@@@oo.  ",
        "    .ooo@@@@o.       .o@@@@@@@@oo.",
        "   oooo@@@@.           o@@@@@@@@oo",
        " ..oo@@@@@o      ..ooo@@@@ooo...  ",
        ".ooo@@@@@@@@@@oo@@@@ooo...ooo.    ",
        "..oo@@@@@@@@@@@@oooo...   o@@oo.  ",
        " ..oo@@@@@@oooo... .@@o. .o@@@oo. ",
        "   ..........      .o@@@@@@@@@@o. ",
        "                     ..oo@@@@oo.. ",
        "               *    .       .   * ",
        "           .        *    .        ",
    ];

    let grad_stops: &[(u32, u32)] = &[
        (palette::BLUE,    palette::SKY),
        (palette::SKY,     palette::TEAL),
        (palette::TEAL,    palette::MAUVE),
        (palette::MAUVE,   palette::LAVENDER),
        (palette::LAVENDER,palette::BLUE),
        (palette::BLUE,    palette::SKY),
        (palette::SKY,     palette::TEAL),
        (palette::TEAL,    palette::MAUVE),
        (palette::MAUVE,   palette::LAVENDER),
        (palette::LAVENDER,palette::BLUE),
        (palette::BLUE,    palette::SKY),
        (palette::SKY,     palette::TEAL),
        (palette::TEAL,    palette::MAUVE),
        (palette::MAUVE,   palette::LAVENDER),
    ];

    let uptime_sec = unsafe { timer_get_uptime_seconds() };
    let days = uptime_sec / 86_400;
    let hours = (uptime_sec % 86_400) / 3600;
    let minutes = (uptime_sec % 3600) / 60;
    let seconds = uptime_sec % 60;
    let uptime_str = if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else {
        format!("{}m {}s", minutes, seconds)
    };

    let total_bytes = crate::pmm::total_memory();
    let used_bytes = crate::pmm::used_memory();
    let total_mib = total_bytes / (1024 * 1024);
    let used_mib = used_bytes / (1024 * 1024);
    let used_percent = if total_bytes == 0 {
        0
    } else {
        used_bytes * 100 / total_bytes
    };
    let mem_str = format!("{} MiB / {} MiB ({}% used)", used_mib, total_mib, used_percent);

    let display_mode = if framebuffer::is_active() {
        format!("{}x{} 32bpp framebuffer", framebuffer::width(), framebuffer::height())
    } else {
        format!("VGA text ({}x{})", vga::get_dimensions().0, vga::get_dimensions().1)
    };

    let paging_str = if crate::vmm::is_paging_enabled() {
        "Enabled"
    } else {
        "Disabled"
    };

    let cpu_brand = crate::cpu::brand_string().unwrap_or_else(|| alloc::format!("i686"));
    let cpu_vendor = crate::cpu::vendor_string().unwrap_or_else(|| alloc::format!("unknown"));

    let print_info = |label: &str, fb_color: u32, vga_color: Color, val: &str| {
        if framebuffer::is_active() {
            framebuffer::set_color(palette::MAUVE, palette::BASE);
            print_pad_right(label, 10);
            framebuffer::set_color(palette::SURFACE1, palette::BASE);
            crate::print!(" | ");
            framebuffer::set_color(fb_color, palette::BASE);
            crate::print!("{}", val);
            framebuffer::set_color(palette::TEXT, palette::BASE);
        } else {
            vga::set_color(Color::LightMagenta, Color::Black);
            print_pad_right(label, 10);
            print_colored!(Color::LightRed, Color::Black, " | ");
            print_colored!(vga_color, Color::Black, "{}", val);
        }
    };

    for r in 0..18 {
        if r < logo.len() {
            if framebuffer::is_active() {
                let (c_l, _) = grad_stops[r % grad_stops.len()];
                framebuffer::set_color(c_l, palette::BASE);
                crate::print!(" {}", logo[r]);
                framebuffer::set_color(palette::TEXT, palette::BASE);
            } else {
                print_colored!(Color::LightCyan, Color::Black, " {}", logo[r]);
            }
        } else {
            crate::print!("                                   ");
        }

        crate::print!("  ");

        match r {
            0 => {
                if framebuffer::is_active() {
                    framebuffer::set_color(palette::YELLOW, palette::BASE);
                    crate::print!("nyxara");
                    framebuffer::set_color(palette::RED, palette::BASE);
                    crate::print!("@");
                    framebuffer::set_color(palette::PEACH, palette::BASE);
                    crate::print!("NyxaraOS");
                    framebuffer::set_color(palette::TEXT, palette::BASE);
                } else {
                    print_colored!(Color::Yellow, Color::Black, "nyxara");
                    print_colored!(Color::LightRed, Color::Black, "@");
                    print_colored!(Color::Yellow, Color::Black, "NyxaraOS");
                }
            }
            1 => {
                if framebuffer::is_active() {
                    framebuffer::set_color(palette::SURFACE1, palette::BASE);
                    crate::print!("----------------------------------------");
                    framebuffer::set_color(palette::TEXT, palette::BASE);
                } else {
                    print_colored!(Color::DarkGray, Color::Black, "----------------------------------------");
                }
            }
            2 => print_info("OS", palette::TEXT, Color::White, "Nyxara OS 2.0.0-hybrid"),
            3 => print_info("Host", palette::TEXT, Color::White, &cpu_vendor),
            4 => print_info("Kernel", palette::GREEN, Color::LightGreen, "NyxaraOS 2.0.0-hybrid"),
            5 => print_info("Architecture", palette::GREEN, Color::LightGreen, "i686 (32-bit)"),
            6 => print_info("Uptime", palette::GREEN, Color::LightGreen, &uptime_str),
            7 => print_info("Memory", palette::SKY, Color::LightCyan, &mem_str),
            8 => print_info("Display", palette::TEAL, Color::LightCyan, &display_mode),
            9 => print_info("Shell", palette::TEAL, Color::LightCyan, "NyxaraSH"),
            10 => print_info("CPU", palette::SAPPHIRE, Color::LightBlue, &cpu_brand),
            11 => print_info("Kernel core", palette::SAPPHIRE, Color::LightBlue, "Rust + C hybrid"),
            12 => print_info("Paging", palette::YELLOW, Color::Yellow, paging_str),
            14 => {
                if framebuffer::is_active() {
                    framebuffer::set_color(palette::SURFACE1, palette::BASE);
                    crate::print!("========================================");
                    framebuffer::set_color(palette::TEXT, palette::BASE);
                } else {
                    print_colored!(Color::DarkGray, Color::Black, "========================================");
                }
            }
            15 => {
                for c in 0u8..8u8 {
                    vga::set_color(Color::Black, Color::from_u8(c));
                    print!("   ");
                }
                if framebuffer::is_active() {
                    framebuffer::set_color(palette::TEXT, palette::BASE);
                } else {
                    vga::set_color(Color::White, Color::Black);
                }
            }
            16 => {
                for c in 8u8..16u8 {
                    vga::set_color(Color::Black, Color::from_u8(c));
                    print!("   ");
                }
                if framebuffer::is_active() {
                    framebuffer::set_color(palette::TEXT, palette::BASE);
                } else {
                    vga::set_color(Color::White, Color::Black);
                }
            }
            17 => {
                if framebuffer::is_active() {
                    framebuffer::set_color(palette::OVERLAY, palette::BASE);
                    crate::print!("lalaufetch v1.1.0  --  galaxy explorer edition  [TrueColor LFB]");
                    framebuffer::set_color(palette::TEXT, palette::BASE);
                } else {
                    print_colored!(Color::DarkGray, Color::Black, "lalaufetch v1.1.0  --  galaxy explorer edition");
                }
            }
            _ => {}
        }

        crate::println!("");
    }

    crate::println!("");
}

pub fn cmd_mway(args: &str) {
    let filename = args.trim();
    if filename.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("mway <filename>");
        println!("  Example: mway catatan.txt");
        return;
    }

    logln!("[mway] Opening file: '{}'", filename);
    crate::editor::open(filename);
    logln!("[mway] Editor closed for: '{}'", filename);
}

pub fn cmd_mouse(args: &str) {
    let sub = args.trim();
    if sub == "test" {
        if !framebuffer::is_active() {
            println!("Mouse test is running in text console. Move mouse or click buttons.");
            println!("Press 'q' or ESC to exit.");
            let mut last_ev = mouse::get_event_count();
            loop {
                if unsafe { keyboard_has_char() } {
                    let key = unsafe { keyboard_getchar() };
                    if key == 27 || key == b'q' as u16 || key == b'Q' as u16 {
                        break;
                    }
                }
                let cur_ev = mouse::get_event_count();
                if cur_ev != last_ev {
                    last_ev = cur_ev;
                    let st = mouse::get_state();
                    println!("Mouse: ({}, {}) | L:{} R:{} M:{} | Events: {}",
                        st.x, st.y, st.left as u8, st.right as u8, st.middle as u8, cur_ev);
                }
            }
            return;
        }

        framebuffer::clear_screen();
        framebuffer::puts(" Nyxara PS/2 Mouse Visual Pointer Test\n");
        framebuffer::puts(" -------------------------------------------------------------\n");
        framebuffer::puts(" Move pointer across screen. Click Left / Right buttons.\n");
        framebuffer::puts(" Press 'q' or ESC to exit back to shell.\n");

        loop {
            if unsafe { keyboard_has_char() } {
                let key = unsafe { keyboard_getchar() };
                if key == 27 || key == b'q' as u16 || key == b'Q' as u16 {
                    break;
                }
            }

            let st = mouse::get_state();
            if st.x >= 0 && st.y >= 0 {
                framebuffer::render_mouse_cursor(st.x as usize, st.y as usize);
            }
        }

        framebuffer::hide_mouse_cursor();
        framebuffer::clear_screen();
        return;
    }

    let st = mouse::get_state();
    let events = mouse::get_event_count();
    print_colored!(Color::LightCyan, Color::Black, "PS/2 Mouse Hardware Status:\n");
    println!("  Position (X, Y) : ({}, {})", st.x, st.y);
    println!("  Buttons         : Left=[{}] Right=[{}] Middle=[{}]",
        if st.left { "PRESSED" } else { "RELEASED" },
        if st.right { "PRESSED" } else { "RELEASED" },
        if st.middle { "PRESSED" } else { "RELEASED" }
    );
    println!("  Total IRQ Events: {}", events);
    if framebuffer::is_active() {
        println!("  Display Bounds  : 0..{} x 0..{}", framebuffer::width(), framebuffer::height());
    } else {
        println!("  Display Bounds  : Text console (80x25)");
    }
    print_colored!(Color::Yellow, Color::Black, "Tips: ");
    println!("Type 'mouse test' for live pointer tracking or 'paint' for drawing canvas.");
}

pub fn cmd_paint() {
    if !framebuffer::is_active() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("'paint' requires active linear framebuffer mode.");
        return;
    }

    let fb_w = framebuffer::width();
    let fb_h = framebuffer::height();
    if fb_w < 100 || fb_h < 100 {
        return;
    }

    const CANVAS_TOP: usize = 36;
    const PALETTE_COUNT: usize = 7;
    let palette_colors: [u32; PALETTE_COUNT] = [
        framebuffer::palette::RED,
        framebuffer::palette::GREEN,
        framebuffer::palette::YELLOW,
        framebuffer::palette::BLUE,
        framebuffer::palette::MAUVE,
        framebuffer::palette::TEXT,
        framebuffer::palette::BASE,
    ];

    let mut current_color_idx: usize = 0;

    framebuffer::clear_screen();
    framebuffer::fill_rect(0, 0, fb_w, fb_h, framebuffer::palette::BASE);
    framebuffer::fill_rect(0, 0, fb_w, CANVAS_TOP - 2, 0x1E1E2E);
    framebuffer::fill_rect(0, CANVAS_TOP - 2, fb_w, 2, framebuffer::palette::SURFACE1);

    let title = "Nyxara Paint | L-Click: Draw | R-Click: Color | C: Clear | 1-7: Palette | ESC: Exit";
    for (i, b) in title.bytes().enumerate() {
        if i + 2 < framebuffer::cols() {
            framebuffer::putchar_at(b, framebuffer::palette::TEXT, 0x1E1E2E, i + 1, 0);
        }
    }

    let draw_palette = |active_idx: usize| {
        for i in 0..PALETTE_COUNT {
            let bx: usize = 16 + i * 36;
            let by: usize = 18;
            let bw: usize = 26;
            let bh: usize = 14;

            let border_c = if i == active_idx { 0xFFFFFF } else { 0x585B70 };
            framebuffer::fill_rect(bx.saturating_sub(1), by.saturating_sub(1), bw + 2, bh + 2, border_c);
            framebuffer::fill_rect(bx, by, bw, bh, palette_colors[i]);
            let digit = b'1' + (i as u8);
            framebuffer::putchar_at(digit, 0xCDD6F4, 0x1E1E2E, (bx + bw + 2) / 8, 1);
        }
    };

    draw_palette(current_color_idx);

    let mut prev_right = false;

    loop {
        if unsafe { keyboard_has_char() } {
            let key = unsafe { keyboard_getchar() };
            if key == 27 || key == b'q' as u16 || key == b'Q' as u16 {
                break;
            } else if key == b'c' as u16 || key == b'C' as u16 {
                framebuffer::hide_mouse_cursor();
                framebuffer::fill_rect(0, CANVAS_TOP, fb_w, fb_h - CANVAS_TOP, framebuffer::palette::BASE);
            } else if key >= b'1' as u16 && key <= b'7' as u16 {
                current_color_idx = (key - b'1' as u16) as usize;
                draw_palette(current_color_idx);
            }
        }

        let st = mouse::get_state();

        if st.right && !prev_right {
            current_color_idx = (current_color_idx + 1) % PALETTE_COUNT;
            draw_palette(current_color_idx);
        }
        prev_right = st.right;

        if st.left {
            let mx = st.x as usize;
            let my = st.y as usize;

            if my >= CANVAS_TOP && mx < fb_w && my < fb_h {
                let bx = mx.saturating_sub(1);
                let by = my.saturating_sub(1);
                framebuffer::fill_rect(bx, by, 3, 3, palette_colors[current_color_idx]);
            } else if my < CANVAS_TOP {
                for i in 0..PALETTE_COUNT {
                    let bx = 16 + i * 36;
                    let by = 18;
                    if mx >= bx && mx <= bx + 28 && my >= by && my <= by + 14 {
                        if current_color_idx != i {
                            current_color_idx = i;
                            draw_palette(current_color_idx);
                        }
                        break;
                    }
                }
            }
        }

        if st.x >= 0 && st.y >= 0 {
            framebuffer::render_mouse_cursor(st.x as usize, st.y as usize);
        }
    }

    framebuffer::hide_mouse_cursor();
    framebuffer::clear_screen();
}
