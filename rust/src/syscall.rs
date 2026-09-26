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
pub const SYS_GETPID: u32 = 20;

extern "C" {
    pub(crate) fn isr_register_handler(n: u8, handler: extern "C" fn(&mut Registers));
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
        SYS_EXIT => sys_exit(arg1 as i32),
        SYS_READ => sys_read(arg1 as i32, arg2 as *mut u8, arg3 as usize),
        SYS_WRITE => sys_write(arg1 as i32, arg2 as *const u8, arg3 as usize),
        SYS_GETPID => sys_getpid(),
        _ => -38, // -ENOSYS
    };

    regs.eax = res as u32;
}

fn sys_exit(code: i32) -> i32 {
    crate::logln!("[Syscall] Process exit requested with code: {}", code);
    0
}

fn sys_read(_fd: i32, _buf: *mut u8, _count: usize) -> i32 {
    // Stdin / file read stub
    0
}

fn sys_write(fd: i32, buf: *const u8, count: usize) -> i32 {
    if buf.is_null() || count == 0 {
        return 0;
    }

    // Unix stdout (fd 1) or stderr (fd 2) -> VGA console & serial log
    if fd == 1 || fd == 2 {
        let slice = unsafe { core::slice::from_raw_parts(buf, count) };
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::print!("{}", s);
            crate::log!("[Syscall stdout] {}", s);
            return count as i32;
        } else {
            for &b in slice {
                crate::print!("{}", b as char);
            }
            return count as i32;
        }
    }

    -9 // -EBADF
}

fn sys_getpid() -> i32 {
    let pid = crate::process::current_pid() as i32;
    crate::logln!("[Syscall] getpid -> {}", pid);
    pid
}
