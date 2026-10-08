use alloc::string::String;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Registers {
    pub ds: u32,
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,
    pub int_no: u32,
    pub err_code: u32,
    pub eip: u32,
    pub cs: u32,
    pub eflags: u32,
    pub useresp: u32,
    pub ss: u32,
}

pub const SYS_EXIT: u32 = 1;
pub const SYS_FORK: u32 = 2;
pub const SYS_READ: u32 = 3;
pub const SYS_WRITE: u32 = 4;
pub const SYS_OPEN: u32 = 5;
pub const SYS_CLOSE: u32 = 6;
pub const SYS_WAITPID: u32 = 7;
pub const SYS_EXECVE: u32 = 11;
pub const SYS_GETPID: u32 = 20;

const EFAULT: i32 = -14;
const ENOENT: i32 = -2;

extern "C" {
    pub(crate) fn isr_register_handler(n: u8, handler: extern "C" fn(&mut Registers));
    fn keyboard_getchar() -> u16;
    fn keyboard_has_char() -> bool;
}

pub fn init() {
    unsafe {
        isr_register_handler(128, syscall_dispatcher);
    }
}

extern "C" fn syscall_dispatcher(regs: &mut Registers) {
    let sys_num = regs.eax;
    let arg1 = regs.ebx;
    let arg2 = regs.ecx;
    let arg3 = regs.edx;

    let res = match sys_num {
        SYS_EXIT => sys_exit(arg1 as i32, regs),
        SYS_FORK => crate::process::fork_current(regs),
        SYS_WAITPID => crate::process::wait_current(regs, arg1 as i32, arg2 as *mut i32),
        SYS_EXECVE => sys_execve(regs, arg1 as *const u8),
        SYS_OPEN => sys_open(arg1 as *const u8, arg2),
        SYS_CLOSE => sys_close(arg1 as i32),
        SYS_READ => sys_read(arg1 as i32, arg2 as *mut u8, arg3 as usize),
        SYS_WRITE => sys_write(arg1 as i32, arg2 as *const u8, arg3 as usize),
        SYS_GETPID => sys_getpid(),
        _ => -38, // -ENOSYS
    };

    regs.eax = res as u32;
}

fn sys_exit(code: i32, regs: &Registers) -> i32 {
    if regs.cs & 3 != 3 {
        crate::logln!("[Syscall] exit refused outside ring 3.");
        return -1;
    }
    crate::process::exit_current(code)
}

fn sys_execve(regs: &mut Registers, path: *const u8) -> i32 {
    let Some(path_str) = read_user_cstr(path) else {
        return EFAULT;
    };
    crate::logln!("[Syscall] execve('{}') requested.", path_str);
    let Some(image) = crate::vfs::read_file(&path_str) else {
        return ENOENT;
    };
    crate::process::exec_current(regs, &image, &path_str)
}

fn read_user_cstr(ptr: *const u8) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut buf = [0u8; 128];
    for i in 0..buf.len() {
        let phys = crate::vmm::get_phys_addr(ptr as usize + i)?;
        let byte = unsafe { (phys as *const u8).read_volatile() };
        if byte == 0 {
            return String::from_utf8(buf[..i].to_vec()).ok();
        }
        buf[i] = byte;
    }
    None
}

fn sys_open(path: *const u8, flags: u32) -> i32 {
    let Some(path_str) = read_user_cstr(path) else {
        return EFAULT;
    };
    if (flags & crate::fd::O_CREAT) != 0 && crate::vfs::read_file(&path_str).is_none() {
        if let Err(e) = crate::vfs::write_file(&path_str, &[]) {
            return e;
        }
    } else if crate::vfs::read_file(&path_str).is_none() {
        return ENOENT;
    }
    match crate::process::current_open(&path_str, flags) {
        Ok(fd) => {
            crate::logln!("[Syscall] open('{}', 0x{:X}) -> fd {}", path_str, flags, fd);
            fd
        }
        Err(e) => e,
    }
}

fn sys_close(fd: i32) -> i32 {
    match crate::process::current_close(fd) {
        Ok(()) => {
            crate::logln!("[Syscall] close(fd {}) -> 0", fd);
            0
        }
        Err(e) => e,
    }
}

fn sys_read(fd: i32, buf: *mut u8, count: usize) -> i32 {
    if buf.is_null() || count == 0 {
        return 0;
    }
    if fd == 0 {
        let mut read_bytes = 0;
        while read_bytes < count {
            if unsafe { keyboard_has_char() } {
                let ch = unsafe { keyboard_getchar() };
                let byte = (ch & 0xFF) as u8;
                if byte != 0 {
                    if let Some(phys) = crate::vmm::get_phys_addr(buf as usize + read_bytes) {
                        unsafe { (phys as *mut u8).write_volatile(byte) };
                        read_bytes += 1;
                        if byte == b'\n' {
                            break;
                        }
                    } else {
                        return if read_bytes > 0 { read_bytes as i32 } else { EFAULT };
                    }
                }
            } else {
                if read_bytes > 0 {
                    break;
                }
                crate::process::yield_now();
            }
        }
        return read_bytes as i32;
    }

    let Some(desc) = crate::process::current_fd_get(fd) else {
        return -9; // -EBADF
    };
    let mut kbuf = [0u8; 512];
    let to_read = count.min(kbuf.len());
    let path = desc.get_path();
    match crate::vfs::read_file_offset(path, desc.offset, &mut kbuf[..to_read]) {
        Ok(n) => {
            for i in 0..n {
                if let Some(phys) = crate::vmm::get_phys_addr(buf as usize + i) {
                    unsafe { (phys as *mut u8).write_volatile(kbuf[i]) };
                } else {
                    return if i > 0 { i as i32 } else { EFAULT };
                }
            }
            crate::process::current_fd_advance(fd, n);
            n as i32
        }
        Err(e) => e,
    }
}

fn sys_write(fd: i32, buf: *const u8, count: usize) -> i32 {
    if buf.is_null() || count == 0 {
        return 0;
    }

    if fd == 1 || fd == 2 {
        let mut bytes = alloc::vec::Vec::with_capacity(count);
        for i in 0..count {
            if let Some(phys) = crate::vmm::get_phys_addr(buf as usize + i) {
                bytes.push(unsafe { (phys as *const u8).read_volatile() });
            } else {
                break;
            }
        }
        if let Ok(s) = core::str::from_utf8(&bytes) {
            crate::print!("{}", s);
            crate::log!("[Syscall stdout] {}", s);
        } else {
            for &b in &bytes {
                crate::print!("{}", b as char);
            }
        }
        return bytes.len() as i32;
    }

    let Some(desc) = crate::process::current_fd_get(fd) else {
        return -9; // -EBADF
    };
    let mut bytes = alloc::vec::Vec::with_capacity(count);
    for i in 0..count {
        if let Some(phys) = crate::vmm::get_phys_addr(buf as usize + i) {
            bytes.push(unsafe { (phys as *const u8).read_volatile() });
        } else {
            break;
        }
    }
    let path = desc.get_path();
    let mut data = crate::vfs::read_file(path).unwrap_or_default();
    let end_pos = desc.offset + bytes.len();
    if data.len() < end_pos {
        data.resize(end_pos, 0);
    }
    data[desc.offset..end_pos].copy_from_slice(&bytes);
    if let Err(e) = crate::vfs::write_file(path, &data) {
        return e;
    }
    crate::process::current_fd_advance(fd, bytes.len());
    bytes.len() as i32
}

fn sys_getpid() -> i32 {
    let pid = crate::process::current_pid() as i32;
    crate::logln!("[Syscall] getpid -> {}", pid);
    pid
}
