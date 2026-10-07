pub mod fs;
pub mod net;
pub mod sys;
pub mod utils;

use crate::vga::Color;
use crate::{print, println, print_colored, logln};
use alloc::format;

pub fn handle_command(cmd: &str) {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return;
    }

    logln!("[Nyxara Shell] Executing command: '{}'", trimmed);

    let mut parts = trimmed.splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    let (command, args) = if command == "sudo" {
        let trimmed_args = args.trim_start();
        let mut sub_parts = trimmed_args.splitn(2, ' ');
        let sub_cmd = sub_parts.next().unwrap_or("");
        let sub_args = sub_parts.next().unwrap_or("");
        (sub_cmd, sub_args)
    } else {
        (command, args)
    };

    match command {
        "sysinfo" => sys::cmd_sysinfo(),
        "free" | "meminfo" => sys::cmd_free(),
        "uptime" => sys::cmd_uptime(),
        "vmm" => sys::cmd_vmm(args),
        "syscall" => sys::cmd_syscall_test(),
        "panic" => sys::cmd_panic(args),
        "reboot" => sys::cmd_reboot(),
        "run" => sys::cmd_run(args),
        "sleep" => sys::cmd_sleep(args),
        "date" => sys::cmd_date(),
        "time" => sys::cmd_time(),

        "pwd" => fs::cmd_pwd(),
        "cd" => fs::cmd_cd(args),
        "mkdir" => fs::cmd_mkdir(args),
        "ls" => fs::cmd_ls(args),
        "rm" => fs::cmd_rm(args),
        "rmdir" => fs::cmd_rmdir(args),
        "cp" => fs::cmd_cp(args),
        "mv" => fs::cmd_mv(args),
        "stat" => fs::cmd_stat(args),
        "cat" => fs::cmd_cat(args),
        "touch" => fs::cmd_touch(args),
        "write" => fs::cmd_write(args),

        "ifconfig" | "netinfo" => net::cmd_ifconfig(args),
        "dhcp" => net::cmd_dhcp(),
        "dns" | "nslookup" => net::cmd_dns(args),
        "ping" => net::cmd_ping(args),
        "arp" => net::cmd_arp(args),
        "netstat" => net::cmd_netstat(),
        "curl" | "fetch" => net::cmd_curl(args),
        "httpd" => net::cmd_httpd(args),
        "nc" => net::cmd_nc(args),

        "help" => utils::cmd_help(),
        "clear" => utils::cmd_clear(),
        "about" | "version" => utils::cmd_about(),
        "echo" => utils::cmd_echo(args),
        "color" => utils::cmd_color(args),
        "calc" => utils::cmd_calc(args),
        "lalaufetch" => utils::cmd_lalaufetch(),
        "mway" => utils::cmd_mway(args),
        "mouse" => utils::cmd_mouse(args),
        "paint" => utils::cmd_paint(),

        "ps" => crate::process::print_tasks(),
        "yield" => crate::process::yield_now(),

        "uname" => {
            if args.trim().contains("-a") {
                println!("NyxaraOS 2.0.0-hybrid #1 SMP i686 GNU/Linux-compat Nyxara");
            } else {
                println!("NyxaraOS");
            }
        }
        "whoami" => println!("root"),
        "hostname" => println!("nyxara"),
        "motd" => {
            if let Some(data) = crate::vfs::read_file("/motd") {
                if let Ok(s) = core::str::from_utf8(&data) {
                    print!("{}", s);
                }
            }
        }
        "exit" | "quit" => {
            println!("Nyxara shell is the root kernel process and cannot exit. Type 'reboot' to restart.");
        }
        "hello" => {
            let _ = sys::run_elf_binary("hello", false);
        }
        _ => {
            let bin_path = format!("/bin/{}", command);
            if crate::vfs::read_file(&bin_path).is_some() {
                let _ = sys::run_elf_binary(command, false);
            } else {
                print_colored!(Color::LightRed, Color::Black, "Error: ");
                println!("Unknown command '{}'. Type 'help' for available commands.", command);
            }
        }
    }
}
