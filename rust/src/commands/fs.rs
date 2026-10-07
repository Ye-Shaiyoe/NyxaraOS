use crate::vga::Color;
use crate::vfs::InodeType;
use crate::{print, println, print_colored};
use super::utils::print_pad_right;

fn split_two_paths(args: &str) -> Option<(&str, &str)> {
    let mut parts = args.split_whitespace();
    let source = parts.next()?;
    let destination = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some((source, destination))
}

pub fn cmd_pwd() {
    println!("{}", crate::vfs::current_dir());
}

pub fn cmd_cd(args: &str) {
    let path = args.trim();
    let target = if path.is_empty() || path == "~" { "/" } else { path };
    if crate::vfs::change_dir(target).is_err() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Directory '{}' not found", target);
    }
}

pub fn cmd_mkdir(args: &str) {
    let trimmed = args.trim();
    let (is_p, dir_path) = if trimmed.starts_with("-p ") {
        (true, trimmed[3..].trim())
    } else if trimmed == "-p" {
        (true, "")
    } else {
        (false, trimmed)
    };

    if dir_path.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("mkdir [-p] <directory>");
        return;
    }

    let res = if is_p {
        crate::vfs::make_dir_p(dir_path)
    } else {
        crate::vfs::make_dir(dir_path)
    };

    if res.is_err() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Failed to create directory '{}'", dir_path);
    }
}

pub fn cmd_rm(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("rm <file>");
        return;
    }
    if crate::vfs::remove_file(path).is_err() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Cannot remove file '{}'", path);
    }
}

pub fn cmd_rmdir(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("rmdir <empty-directory>");
        return;
    }
    if crate::vfs::remove_dir(path).is_err() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Cannot remove directory '{}' (it must be empty and not be the current directory)", path);
    }
}

pub fn cmd_cp(args: &str) {
    let Some((source, destination)) = split_two_paths(args.trim()) else {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("cp <source> <destination>");
        return;
    };
    if crate::vfs::copy_file(source, destination).is_err() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Cannot copy '{}' to '{}'", source, destination);
    }
}

pub fn cmd_mv(args: &str) {
    let Some((source, destination)) = split_two_paths(args.trim()) else {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("mv <source> <destination>");
        return;
    };
    if crate::vfs::move_file(source, destination).is_err() {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Cannot move '{}' to '{}'", source, destination);
    }
}

pub fn cmd_stat(args: &str) {
    let path = args.trim();
    let target = if path.is_empty() { "." } else { path };
    match crate::vfs::stat(target) {
        Some((name, kind, size)) => {
            println!("Path : {}", name);
            println!("Type : {:?}", kind);
            println!("Size : {} bytes", size);
        }
        None => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("Path '{}' not found", target);
        }
    }
}

pub fn cmd_ls(args: &str) {
    let mut target_path = "";
    for part in args.split_whitespace() {
        if !part.starts_with('-') {
            target_path = part;
            break;
        }
    }

    match crate::vfs::list_dir(target_path) {
        Ok(entries) => {
            print_colored!(Color::LightCyan, Color::Black, "VFS: {}\n", if target_path.is_empty() { "." } else { target_path });
            if entries.is_empty() {
                println!("  (empty)");
                return;
            }
            for (name, size, kind) in entries {
                print!("  ");
                print_pad_right(&name, 16);
                if kind == InodeType::Directory {
                    println!("<DIR>");
                } else {
                    println!("{} bytes", size);
                }
            }
        }
        Err(_) => {
            if let Some((_, kind, size)) = crate::vfs::stat(target_path) {
                if kind == InodeType::File {
                    print_colored!(Color::LightCyan, Color::Black, "VFS: {}\n", target_path);
                    print!("  ");
                    let display_name = target_path.rsplit('/').next().unwrap_or(target_path);
                    print_pad_right(display_name, 16);
                    println!("{} bytes", size);
                    return;
                }
            }
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("Path '{}' not found", if target_path.is_empty() { "." } else { target_path });
        }
    }
}

pub fn cmd_cat(args: &str) {
    let file = args.trim();
    if file.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("cat <filename>");
        return;
    }

    if let Some((_, kind, _)) = crate::vfs::stat(file) {
        if kind == InodeType::Directory {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("'{}' is a directory", file);
            return;
        }
    }

    match crate::vfs::read_file(file) {
        Some(data) => {
            if let Ok(s) = core::str::from_utf8(&data) {
                print!("{}", s);
                if !s.ends_with('\n') {
                    println!("");
                }
            } else {
                for b in data {
                    print!("{:02X} ", b);
                }
                println!("");
            }
        }
        None => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("File '{}' not found", file);
        }
    }
}

pub fn cmd_touch(args: &str) {
    let file = args.trim();
    if file.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("touch <filename>");
        return;
    }

    if crate::vfs::read_file(file).is_some() {
        return;
    }

    if let Err(_) = crate::vfs::write_file(file, b"") {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Failed to create file '{}'", file);
    }
}

pub fn cmd_write(args: &str) {
    let mut parts = args.trim().splitn(2, ' ');
    let file = parts.next().unwrap_or("");
    let text = parts.next().unwrap_or("");

    if file.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("write <filename> <content>");
        return;
    }

    let mut data = alloc::vec::Vec::new();
    data.extend_from_slice(text.as_bytes());
    data.push(b'\n');

    if let Err(_) = crate::vfs::write_file(file, &data) {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Failed to write to file '{}'", file);
    }
}
