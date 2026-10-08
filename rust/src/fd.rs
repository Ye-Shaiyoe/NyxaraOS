pub const MAX_FDS: usize = 8;

pub const O_RDONLY: u32 = 0;
pub const O_WRONLY: u32 = 1;
pub const O_RDWR: u32 = 2;
pub const O_CREAT: u32 = 0x40;
pub const O_TRUNC: u32 = 0x200;
pub const O_APPEND: u32 = 0x400;

const EBADF: i32 = -9;
const EMFILE: i32 = -24;

#[derive(Copy, Clone)]
pub struct FileDesc {
    pub path: [u8; 32],
    pub offset: usize,
    pub flags: u32,
}

impl FileDesc {
    pub fn get_path(&self) -> &str {
        let len = self.path.iter().position(|&b| b == 0).unwrap_or(32);
        core::str::from_utf8(&self.path[..len]).unwrap_or("")
    }

    pub fn set_path(&mut self, p: &str) {
        self.path = [0; 32];
        let bytes = p.as_bytes();
        let len = bytes.len().min(31);
        self.path[..len].copy_from_slice(&bytes[..len]);
    }
}

#[derive(Copy, Clone)]
pub struct FdTable {
    fds: [Option<FileDesc>; MAX_FDS],
}

impl FdTable {
    pub const fn new() -> Self {
        const NONE: Option<FileDesc> = None;
        Self { fds: [NONE; MAX_FDS] }
    }

    pub fn open(&mut self, path: &str, flags: u32) -> Result<i32, i32> {
        for i in 3..MAX_FDS {
            if self.fds[i].is_none() {
                let mut desc = FileDesc {
                    path: [0; 32],
                    offset: 0,
                    flags,
                };
                desc.set_path(path);
                self.fds[i] = Some(desc);
                return Ok(i as i32);
            }
        }
        Err(EMFILE)
    }

    pub fn close(&mut self, fd: i32) -> Result<(), i32> {
        let idx = fd as usize;
        if idx < 3 || idx >= MAX_FDS {
            return Err(EBADF);
        }
        if self.fds[idx].is_none() {
            return Err(EBADF);
        }
        self.fds[idx] = None;
        Ok(())
    }

    pub fn get(&self, fd: i32) -> Option<&FileDesc> {
        let idx = fd as usize;
        if idx >= MAX_FDS {
            return None;
        }
        self.fds[idx].as_ref()
    }

    pub fn get_mut(&mut self, fd: i32) -> Option<&mut FileDesc> {
        let idx = fd as usize;
        if idx >= MAX_FDS {
            return None;
        }
        self.fds[idx].as_mut()
    }

    pub fn close_all(&mut self) {
        for i in 3..MAX_FDS {
            self.fds[i] = None;
        }
    }
}
